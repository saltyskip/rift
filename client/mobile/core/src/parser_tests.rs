use super::*;

fn hosts() -> Vec<String> {
    vec!["go.example.com".into(), "api.riftl.ink".into()]
}

#[test]
fn clipboard_url_custom_domain() {
    assert_eq!(
        parse_clipboard_link("https://go.example.com/my-link", &hosts()),
        Some("my-link".into())
    );
}

#[test]
fn clipboard_url_primary_domain() {
    assert_eq!(
        parse_clipboard_link("https://api.riftl.ink/r/ABC123", &hosts()),
        Some("ABC123".into())
    );
}

#[test]
fn clipboard_url_with_query() {
    assert_eq!(
        parse_clipboard_link("https://go.example.com/summer-sale?ref=twitter", &hosts()),
        Some("summer-sale".into())
    );
}

#[test]
fn clipboard_rejects_untrusted_host() {
    // An unrelated URL left on the clipboard must not be mis-attributed.
    assert_eq!(
        parse_clipboard_link("https://othersite.com/promo", &hosts()),
        None
    );
    assert_eq!(
        parse_clipboard_link("https://nytimes.com/login", &hosts()),
        None
    );
}

#[test]
fn clipboard_host_case_insensitive_and_ignores_port() {
    assert_eq!(
        parse_clipboard_link("https://GO.EXAMPLE.COM/My-Link", &hosts()),
        Some("My-Link".into())
    );
    assert_eq!(
        parse_clipboard_link("http://go.example.com:8080/abc", &hosts()),
        Some("abc".into())
    );
}

#[test]
fn clipboard_empty_allowed_hosts_rejects_all() {
    assert_eq!(
        parse_clipboard_link("https://go.example.com/my-link", &[]),
        None
    );
}

#[test]
fn clipboard_invalid() {
    assert_eq!(parse_clipboard_link("", &hosts()), None);
    assert_eq!(parse_clipboard_link("just-some-text", &hosts()), None);
    assert_eq!(parse_clipboard_link("https://", &hosts()), None);
    // Bare domain with no path segment is not a link.
    assert_eq!(
        parse_clipboard_link("https://go.example.com", &hosts()),
        None
    );
    // The legacy `rift:` scheme is no longer recognized.
    assert_eq!(parse_clipboard_link("rift:ABC123", &hosts()), None);
}

#[test]
fn referrer_valid() {
    assert_eq!(
        parse_referrer_link("rift_link=ABC123"),
        Some("ABC123".into())
    );
    assert_eq!(
        parse_referrer_link("utm_source=google&rift_link=ABC123&utm_medium=cpc"),
        Some("ABC123".into())
    );
}

#[test]
fn referrer_invalid() {
    assert_eq!(parse_referrer_link("utm_source=google"), None);
    assert_eq!(parse_referrer_link(""), None);
    assert_eq!(parse_referrer_link("rift_link="), None);
}
