//! Equalizer settings and routing. Audio processing lives with each playback source.
use serde_json::{json, Value};
use wry::WebView;

pub(crate) fn adjust(eq: &mut Value, cmd: &str, index: usize, value: f64, msg: &Value) {
    match cmd {
        "eq_change" => {
            if index == 0 {
                eq["preamp"] = json!(value.clamp(-12.0, 12.0));
            } else if index <= 9 {
                eq["bands"][index - 1] = json!(value.clamp(-12.0, 12.0));
            }
        }
        "eq_enabled" => eq["enabled"] = json!(msg["value"].as_bool().unwrap_or(false)),
        "eq_preset" => {
            let gains = match msg["value"].as_str().unwrap_or("") {
                "rock" => vec![5., 3., -2., -3., 1., 3., 4., 4., 3.],
                "bass" => vec![8., 5., 2., 0., 0., 0., 0., 0., 0.],
                "voice" => vec![-5., -3., -1., 2., 4., 4., 1., -2., -3.],
                _ => vec![0.; 9],
            };
            eq["bands"] = json!(gains);
            eq["preamp"] = json!(0);
        }
        _ => {}
    }
}
pub(crate) fn apply(eq: &Value, local: bool, youtube: &WebView, library: &WebView) {
    let (view, script) = if local {
        (
            library,
            format!("window.localAudio&&window.localAudio.apply({eq});"),
        )
    } else {
        (
            youtube,
            format!("window.braveampEq&&window.braveampEq.configure({eq});"),
        )
    };
    let _ = view.evaluate_script(&script);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_gain_and_keeps_other_bands() {
        let mut e = json!({"preamp":0,"bands":[0,0,0,0,0,0,0,0,0],"enabled":false});
        adjust(&mut e, "eq_change", 5, 90.0, &json!({}));
        assert_eq!(e["bands"][4], 12.0);
        assert_eq!(e["bands"][3], 0);
    }
}
