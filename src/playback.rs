//! Playback source validation and transport; no window or library ownership.
use serde_json::Value;
use wry::WebView;
pub(crate) fn video_id(u: &str) -> Option<String> {
    let u = url::Url::parse(u).ok()?;
    if u.scheme() != "https" {
        return None;
    }
    let host = u.host_str()?;
    let id = if host == "youtu.be" {
        u.path_segments()?.next()?.to_string()
    } else if ["youtube.com", "www.youtube.com", "m.youtube.com"].contains(&host) {
        u.query_pairs().find(|(k, _)| k == "v")?.1.into_owned()
    } else {
        return None;
    };
    if id.len() == 11
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        Some(id)
    } else {
        None
    }
}
pub(crate) fn js_video(web: &WebView, action: &str, value: f64) {
    let script = match action {
        "play" => {
            "const v=document.querySelector('video');if(v)v.play().catch(()=>{});".to_string()
        }
        "pause" => "window.braveampCancelStartup?.();const v=document.querySelector('video');if(v)v.pause();".to_string(),
        "stop" => {
            "window.braveampCancelStartup?.();const v=document.querySelector('video');if(v){v.pause();v.currentTime=0;}".to_string()
        }
        "volume" => format!(
            "const v=document.querySelector('video');if(v)v.volume={};",
            value.clamp(0.0, 1.0)
        ),
        "seek" => format!(
            "const v=document.querySelector('video');if(v)v.currentTime={};",
            value.max(0.0)
        ),
        _ => return,
    };
    let _ = web.evaluate_script(&format!("(()=>{{{script}}})()"));
}

pub(crate) fn load(item: &Value, web: &WebView, lib: &WebView) -> bool {
    if item["kind"] == "local" {
        if let Some(id) = item["id"].as_u64() {
            js_video(web, "pause", 0.0);
            let _ = lib.evaluate_script(&format!(
                "window.localAudio&&window.localAudio.play({id}).catch(e=>console.error(e));"
            ));
            return true;
        }
        return false;
    }
    let _ = lib.evaluate_script("window.localAudio&&window.localAudio.pause();");
    if let Some(id) = item.get("id").and_then(Value::as_str) {
        if id.len() == 11
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            let _ = web.load_url(&format!("https://m.youtube.com/watch?v={id}"));
            return true;
        }
    }
    false
}

/// Choose the next queue item; caller supplies entropy for reproducible tests.
pub(crate) fn next_index(
    current: Option<usize>,
    len: usize,
    shuffle: bool,
    repeat: bool,
    entropy: usize,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let i = current.unwrap_or(0).min(len - 1);
    let next = if shuffle && len > 1 {
        (i + 1 + entropy % (len - 1)) % len
    } else {
        i + 1
    };
    if next < len || repeat {
        Some(next % len)
    } else {
        None
    }
}
pub(crate) fn next_from_state(current: Option<usize>, len: usize, state: &Value) -> Option<usize> {
    let entropy = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    next_index(
        current,
        len,
        state["shuffle"].as_bool().unwrap_or(false),
        state["repeat"].as_bool().unwrap_or(false),
        entropy,
    )
}
#[cfg(test)]
mod tests {
    use super::{next_index, video_id};
    #[test]
    fn accepts_only_valid_youtube_ids() {
        assert_eq!(
            video_id("https://youtu.be/a5uQMwRMHcs").as_deref(),
            Some("a5uQMwRMHcs")
        );
        assert!(video_id("https://youtube.com.evil.example/watch?v=a5uQMwRMHcs").is_none());
        assert!(video_id("http://www.youtube.com/watch?v=a5uQMwRMHcs").is_none());
    }
    #[test]
    fn next_respects_end_repeat_and_shuffle() {
        assert_eq!(next_index(Some(1), 2, false, false, 0), None);
        assert_eq!(next_index(Some(1), 2, false, true, 0), Some(0));
        assert_eq!(next_index(Some(0), 2, true, false, 99), Some(1));
        assert_eq!(next_index(None, 0, true, true, 0), None);
    }
}
