//! Brave's Rust filtering engine connected to WebView2's network-request event.
//! Uses a bundled snapshot of Brave default rules and resources. It is not the full Shields implementation and does not automatically update the snapshot.
use adblock::{
    lists::{FilterSet, ParseOptions},
    request::Request,
    resources::{PermissionMask, Resource},
    Engine,
};
use std::rc::Rc;
#[cfg(target_os = "windows")]
use webview2_com::{
    take_pwstr, Microsoft::Web::WebView2::Win32::*, WebResourceRequestedEventHandler,
};
#[cfg(target_os = "windows")]
use windows::core::{Interface, HSTRING, PWSTR};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::IStream;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::SHCreateMemStream;
#[cfg(target_os = "windows")]
use wry::{WebView, WebViewExtWindows};

fn make_engine() -> Engine {
    let mut filters = FilterSet::new(false);
    // These files are the signed default and first-party Brave component snapshots.
    // Only these official bundled lists get the permission mask used by Brave.
    let official = ParseOptions {
        permissions: PermissionMask::from_bits(1),
        ..Default::default()
    };
    filters.add_filter_list(
        include_str!("filters/brave-default.txt").to_string(),
        official,
    );
    filters.add_filter_list(
        include_str!("filters/brave-firstparty.txt").to_string(),
        official,
    );
    let mut engine = Engine::new_with_filter_set(filters);
    let resources: Vec<Resource> =
        serde_json::from_str(include_str!("filters/brave-resources.json"))
            .expect("El paquete de recursos de Brave debe ser válido");
    engine.use_resources(resources);
    engine
}
thread_local! { static FILTER_ENGINE: Rc<Engine> = Rc::new(make_engine()); }

/// Runs the matching Brave scriptlets in the page's main JS world at document start.
/// Install this through Wry's with_initialization_script, before loading YouTube.
/// Each hostname gets only the rules the filter engine matched for that hostname.
pub fn initialization_script() -> String {
    FILTER_ENGINE.with(|engine| {
        let mut output = String::from("(()=>{const h=location.hostname;\n");
        for host in ["www.youtube.com", "m.youtube.com", "youtube.com"] {
            let url = format!("https://{host}/watch");
            let r = engine.url_cosmetic_resources(&url);
            let host_json = serde_json::to_string(host).expect("host válido");
            let css = r.hide_selectors.iter().map(|selector| format!("{selector}{{display:none!important}}"))
                .collect::<Vec<_>>().join("\n");
            let css_json = serde_json::to_string(&css).expect("CSS válido");
            output.push_str(&format!("if(h==={host_json}){{\n{}\nconst css={css_json};if(css){{const add=()=>{{if(!document.documentElement)return;const s=document.createElement('style');s.textContent=css;document.documentElement.appendChild(s)}};if(document.documentElement)add();else document.addEventListener('DOMContentLoaded',add,{{once:true}});}}\n}}\n", r.injected_script));
        }
        output.push_str("})();");
        output
    })
}

// Brave supplies redirect bodies as data URLs; decode them without pulling in
// another dependency so that blocking a request can return the intended shim.
fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u8;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let n = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buf = (buf << 6) | (n as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    Some(out)
}
fn redirect_payload(data: &str) -> Option<(&str, Vec<u8>)> {
    let (metadata, encoded) = data.strip_prefix("data:")?.split_once(',')?;
    let mime = metadata.strip_suffix(";base64")?;
    if mime.is_empty()
        || !mime
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/+-._".contains(&c))
    {
        return None;
    }
    Some((mime, decode_base64(encoded)?))
}

#[cfg(target_os = "windows")]
fn request_type(context: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> &'static str {
    match context {
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => "document",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST => "xmlhttprequest",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH => "xmlhttprequest",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET => "websocket",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_EVENT_SOURCE => "other",
        _ => "other",
    }
}

/// Install immediately after building the YouTube webview, before navigating it.
/// Returns an event registration token; WebView2 retains the callback.
#[cfg(target_os = "windows")]
pub fn install(webview: &WebView) -> windows::core::Result<i64> {
    let engine = FILTER_ENGINE.with(Rc::clone);
    let inner = webview.webview();
    let environment = webview.environment();
    let filter = HSTRING::from("*");
    let mut token: i64 = 0;
    unsafe {
        // Newer WebView2 includes worker and iframe traffic. Fall back when needed.
        if let Ok(v22) = inner.cast::<ICoreWebView2_22>() {
            v22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                &filter,
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )?;
        } else {
            inner.AddWebResourceRequestedFilter(&filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?;
        }
        inner.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                let Some(args) = args else {
                    return Ok(());
                };
                let req = args.Request()?;
                let mut url_ptr = PWSTR::null();
                req.Uri(&mut url_ptr)?;
                let url = take_pwstr(url_ptr);
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Ok(());
                }
                let mut method_ptr = PWSTR::null();
                req.Method(&mut method_ptr)?;
                let method = take_pwstr(method_ptr);
                let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                args.ResourceContext(&mut context)?;
                let source = if let Some(view) = sender {
                    let mut ptr = PWSTR::null();
                    if view.Source(&mut ptr).is_ok() && !ptr.is_null() {
                        take_pwstr(ptr)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                // Source is the top-level URL; WebView2 doesn't expose exact initiator here.
                let source = if source.starts_with("http") {
                    source.as_str()
                } else {
                    "https://www.youtube.com/"
                };
                if let Ok(request) = Request::new(&url, source, request_type(context), &method) {
                    let result = engine.check_network_request(&request);
                    if result.should_block() {
                        // Network-only prototype: redirect resources and scriptlet injection are
                        // intentionally not implemented; a blocked response has an empty body.
                        if let Some((mime, bytes)) =
                            result.redirect.as_deref().and_then(redirect_payload)
                        {
                            if let Some(stream) = SHCreateMemStream(Some(&bytes)) {
                                let headers = HSTRING::from(format!(
                                    "Content-Type: {mime}\r\nCache-Control: no-store\r\n"
                                ));
                                let response = environment.CreateWebResourceResponse(
                                    &stream,
                                    200,
                                    &HSTRING::from("OK"),
                                    &headers,
                                )?;
                                args.SetResponse(&response)?;
                                return Ok(());
                            }
                        }
                        let response = environment.CreateWebResourceResponse(
                            None::<&IStream>,
                            403,
                            &HSTRING::from("Blocked"),
                            &HSTRING::from("Cache-Control: no-store\r\n"),
                        )?;
                        args.SetResponse(&response)?;
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_block_known_ad_request_without_blocking_video_host() {
        let mut set = FilterSet::new(false);
        set.add_filter_list(
            include_str!("filters/easylist.txt").to_string(),
            ParseOptions::default(),
        );
        set.add_filter_list(
            include_str!("filters/brave-unbreak.txt").to_string(),
            ParseOptions::default(),
        );
        let engine = Engine::new_with_filter_set(set);
        let ad = Request::new(
            "https://googlesyndication.com/pagead/ad.js",
            "https://www.youtube.com/",
            "script",
            "GET",
        )
        .unwrap();
        let video = Request::new(
            "https://www.youtube.com/watch?v=abcdefghijk",
            "https://www.youtube.com/",
            "document",
            "GET",
        )
        .unwrap();
        assert!(engine.check_network_request(&ad).should_block());
        assert!(!engine.check_network_request(&video).should_block());
    }

    #[test]
    fn brave_scriptlets_for_youtube_are_available_and_scoped() {
        let script = initialization_script();
        assert!(script.contains("playerAds"));
        assert!(script.contains("adPlacements"));
        assert!(script.contains("m.youtube.com"));
        assert!(script.contains("www.youtube.com"));
    }
    #[test]
    fn document_start_script_has_valid_javascript_syntax() {
        use std::{
            io::Write,
            process::{Command, Stdio},
        };
        let Ok(mut child) = Command::new("node")
            .arg("--check")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        else {
            return;
        };
        child
            .stdin
            .take()
            .unwrap()
            .write_all(initialization_script().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    #[test]
    fn decodes_the_redirects_produced_by_brave() {
        let (mime, bytes) =
            redirect_payload("data:application/javascript;base64,YWxlcnQoMSk7").unwrap();
        assert_eq!(mime, "application/javascript");
        assert_eq!(bytes, b"alert(1);");
    }
    #[test]
    fn exceptions_are_honored() {
        let mut set = FilterSet::new(false);
        set.add_filter_list(
            "||example.com/ads/\n@@||example.com/ads/allowed.js".to_string(),
            ParseOptions::default(),
        );
        let engine = Engine::new_with_filter_set(set);
        let allowed = Request::new(
            "https://example.com/ads/allowed.js",
            "https://youtube.com/",
            "script",
            "GET",
        )
        .unwrap();
        let blocked = Request::new(
            "https://example.com/ads/blocked.js",
            "https://youtube.com/",
            "script",
            "GET",
        )
        .unwrap();
        assert!(!engine.check_network_request(&allowed).should_block());
        assert!(engine.check_network_request(&blocked).should_block());
    }
}
