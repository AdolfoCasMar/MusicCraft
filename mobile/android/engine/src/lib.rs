//! Experimental Android adapter for Brave's adblock-rust engine.
//! Network interception is only one part of Brave Shields.
use adblock::{lists::{FilterSet, ParseOptions}, request::Request, resources::{PermissionMask, Resource}, Engine};
use jni::{objects::{JClass, JString}, sys::{jboolean, jstring, JNI_FALSE, JNI_TRUE}, JNIEnv};
use std::sync::OnceLock;

fn make_engine() -> Engine {
    let mut filters = FilterSet::new(false);
    let official = ParseOptions { permissions: PermissionMask::from_bits(1), ..Default::default() };
    filters.add_filter_list(include_str!("../filters/brave-default.txt").to_owned(), official);
    filters.add_filter_list(include_str!("../filters/brave-firstparty.txt").to_owned(), official);
    let mut engine = Engine::new_with_filter_set(filters);
    let resources: Vec<Resource> = serde_json::from_str(include_str!("../filters/brave-resources.json"))
        .expect("Brave resources must be valid JSON");
    engine.use_resources(resources);
    engine
}

fn engine() -> &'static Engine {
    static INSTANCE: OnceLock<Engine> = OnceLock::new();
    INSTANCE.get_or_init(make_engine)
}

fn should_block(url: &str, source: &str, kind: &str, method: &str) -> bool {
    if let Ok(request) = Request::new(url, source, kind, method) {
        let result = engine().check_network_request(&request);
        // Resource redirects require a replacement payload. Until that is
        // implemented, leave them alone rather than break the requested page.
        result.should_block() && result.redirect.is_none()
    } else { false }
}

fn cosmetic_script() -> String {
    let mut output = String::from("(()=>{if(window.__musiccraftFilters)return;window.__musiccraftFilters=true;const h=location.hostname;\n");
    for host in ["www.youtube.com", "m.youtube.com", "youtube.com"] {
        let r = engine().url_cosmetic_resources(&format!("https://{host}/watch"));
        let css = r.hide_selectors.iter().map(|selector|format!("{selector}{{display:none!important}}"))
            .collect::<Vec<_>>().join("\n");
        let css_json = serde_json::to_string(&css).expect("valid CSS string");
        output.push_str(&format!("if(h==={}){{\n{}\nconst css={css_json};if(css){{const add=()=>{{if(!document.documentElement)return;const s=document.createElement('style');s.textContent=css;document.documentElement.appendChild(s)}};if(document.documentElement)add();else document.addEventListener('DOMContentLoaded',add,{{once:true}});}}\n}}\n",serde_json::to_string(host).unwrap(),r.injected_script));
    }
    output.push_str("})();");
    output
}

#[no_mangle]
pub extern "system" fn Java_org_musiccraft_player_BraveFilter_nativeShouldBlock(
    mut env: JNIEnv, _class: JClass, url: JString, source: JString, kind: JString, method: JString,
) -> jboolean {
    let strings = (|| -> jni::errors::Result<(String,String,String,String)> {
        Ok((env.get_string(&url)?.into(), env.get_string(&source)?.into(),
            env.get_string(&kind)?.into(), env.get_string(&method)?.into()))
    })();
    match strings {
        Ok((url, source, kind, method)) if should_block(&url, &source, &kind, &method) => JNI_TRUE,
        _ => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_org_musiccraft_player_BraveFilter_nativeCosmeticScript(
    env: JNIEnv, _class: JClass,
) -> jstring {
    match env.new_string(cosmetic_script()) {
        Ok(result) => result.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leaves_youtube_page_available_and_rejects_malformed_url() {
        assert!(!should_block("https://m.youtube.com/watch?v=abcdefghijk", "https://m.youtube.com/", "document", "GET"));
        assert!(!should_block("invalid-url", "https://m.youtube.com/", "other", "GET"));
    }
    #[test]
    fn blocks_a_known_ad_request() {
        assert!(should_block("https://googlesyndication.com/pagead/ad.js", "https://m.youtube.com/", "script", "GET"));
    }
    #[test]
    fn loads_official_youtube_scriptlets() {
        let script = cosmetic_script();
        assert!(script.contains("adPlacements"));
        assert!(script.contains("m.youtube.com"));
        use std::{io::Write, process::{Command, Stdio}};
        let mut child = Command::new("node").arg("--check").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().expect("Node is required to check the bundled script");
        child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
}
