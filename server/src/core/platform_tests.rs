use super::{detect_os, detect_os_from_ua, Os};
use axum::http::HeaderMap;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut h = HeaderMap::new();
    for (k, v) in pairs {
        h.insert(
            axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
            v.parse().unwrap(),
        );
    }
    h
}

// ── as_str stability (don't break historical analytics) ──

#[test]
fn as_str_values_are_stable() {
    assert_eq!(Os::Ios.as_str(), "ios");
    assert_eq!(Os::Android.as_str(), "android");
    assert_eq!(Os::Other.as_str(), "other");
    assert_eq!(Os::Mac.as_str(), "macos");
    assert_eq!(Os::Windows.as_str(), "windows");
}

// ── Client hint takes precedence ──

#[test]
fn client_hint_maps_each_os() {
    for (hint, expected) in [
        ("\"macOS\"", Os::Mac),
        ("\"Windows\"", Os::Windows),
        ("\"Android\"", Os::Android),
        ("\"iOS\"", Os::Ios),
    ] {
        let h = headers(&[("sec-ch-ua-platform", hint)]);
        assert_eq!(detect_os(&h), expected, "hint {hint}");
    }
}

#[test]
fn client_hint_linux_or_unknown_falls_through_to_ua() {
    // Linux hint + a Windows UA → UA decides (Linux is the catch-all that
    // should defer rather than win).
    let h = headers(&[
        ("sec-ch-ua-platform", "\"Linux\""),
        ("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
    ]);
    assert_eq!(detect_os(&h), Os::Windows);
}

#[test]
fn client_hint_wins_over_ua() {
    // Windows hint but a macOS-looking UA → hint wins.
    let h = headers(&[
        ("sec-ch-ua-platform", "\"Windows\""),
        (
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        ),
    ]);
    assert_eq!(detect_os(&h), Os::Windows);
}

// ── UA fallback (Safari/Firefox/privacy clients send no hint) ──

#[test]
fn ua_fallback_macos_safari() {
    let h = headers(&[(
        "user-agent",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605 Version/17 Safari/605",
    )]);
    assert_eq!(detect_os(&h), Os::Mac);
}

#[test]
fn ua_fallback_windows() {
    assert_eq!(
        detect_os_from_ua("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
        Os::Windows
    );
}

#[test]
fn ua_mobile_before_desktop() {
    // iOS UA contains "like Mac OS X" — must resolve to Ios, not Mac.
    assert_eq!(
        detect_os_from_ua("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)"),
        Os::Ios
    );
    // Android UA contains "Linux" — must resolve to Android, not Other.
    assert_eq!(
        detect_os_from_ua("Mozilla/5.0 (Linux; Android 14; Pixel 8)"),
        Os::Android
    );
}

#[test]
fn ua_ipad_and_ipod() {
    assert_eq!(
        detect_os_from_ua("Mozilla/5.0 (iPad; CPU OS 17_0)"),
        Os::Ios
    );
    assert_eq!(detect_os_from_ua("Mozilla/5.0 (iPod touch)"), Os::Ios);
}

#[test]
fn ua_bot_and_unknown_are_other() {
    assert_eq!(
        detect_os_from_ua("Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)"),
        Os::Other
    );
    assert_eq!(detect_os_from_ua("curl/8.4.0"), Os::Other);
}

#[test]
fn no_headers_is_other() {
    assert_eq!(detect_os(&HeaderMap::new()), Os::Other);
}
