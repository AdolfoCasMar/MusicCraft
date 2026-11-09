//! Classify navigations without touching cookie values or making a choice for the user.
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKind {
    /// A YouTube/Google consent screen that the user must resolve.
    Consent,
    /// A normal YouTube page (may still show an in-page consent dialog).
    Youtube,
    /// Anything else: do not interpret it as a consent signal.
    Other,
}

fn youtube_host(host: &str) -> bool {
    matches!(host, "youtube.com" | "www.youtube.com" | "m.youtube.com")
}

pub fn classify_navigation(input: &str) -> NavigationKind {
    let Ok(url) = Url::parse(input) else {
        return NavigationKind::Other;
    };
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return NavigationKind::Other;
    }
    let Some(host) = url.host_str() else {
        return NavigationKind::Other;
    };
    if host == "consent.youtube.com" {
        return NavigationKind::Consent;
    }
    // Google can redirect its consent page to YouTube through an encoded
    // `continue` URL. Never trust a lookalike host or an arbitrary continue URL.
    if host == "consent.google.com" {
        let for_youtube = url.query_pairs().any(|(key, val)| {
            key == "continue"
                && Url::parse(&val).ok().is_some_and(|dest| {
                    dest.scheme() == "https"
                        && dest.username().is_empty()
                        && dest.password().is_none()
                        && dest.host_str().is_some_and(youtube_host)
                })
        });
        return if for_youtube {
            NavigationKind::Consent
        } else {
            NavigationKind::Other
        };
    }
    if youtube_host(host) {
        NavigationKind::Youtube
    } else {
        NavigationKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifies_youtube_consent_without_exposing_choice() {
        assert_eq!(classify_navigation("https://consent.youtube.com/m?continue=https%3A%2F%2Fm.youtube.com%2Fwatch%3Fv%3Dabc"), NavigationKind::Consent);
        assert_eq!(
            classify_navigation(
                "https://consent.google.com/m?continue=https%3A%2F%2Fwww.youtube.com%2Fresults"
            ),
            NavigationKind::Consent
        );
    }
    #[test]
    fn only_allows_verified_secure_youtube_hosts() {
        assert_eq!(
            classify_navigation("https://m.youtube.com/watch?v=abcdefghijk"),
            NavigationKind::Youtube
        );
        assert_eq!(
            classify_navigation("http://consent.youtube.com/m"),
            NavigationKind::Other
        );
        assert_eq!(
            classify_navigation("https://consent.youtube.com.evil.example/m"),
            NavigationKind::Other
        );
        assert_eq!(
            classify_navigation(
                "https://consent.google.com/m?continue=https%3A%2F%2Fyoutube.com.evil.example"
            ),
            NavigationKind::Other
        );
        assert_eq!(
            classify_navigation("https://youtube.com@evil.example/"),
            NavigationKind::Other
        );
        assert_eq!(classify_navigation("not a url"), NavigationKind::Other);
    }
}
