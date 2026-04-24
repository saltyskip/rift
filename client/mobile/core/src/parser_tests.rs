use super::*;

#[test]
fn clipboard_url_custom_domain() {
    assert_eq!(
        parse_clipboard_link("https://go.example.com/my-link"),
        Some("my-link".into())
    );
}

#[test]
fn clipboard_url_primary_domain() {
    assert_eq!(
        parse_clipboard_link("https://api.riftl.ink/r/ABC123"),
        Some("ABC123".into())
    );
}

#[test]
fn clipboard_url_with_query() {
    assert_eq!(
        parse_clipboard_link("https://go.example.com/summer-sale?ref=twitter"),
        Some("summer-sale".into())
    );
}

#[test]
fn clipboard_legacy_format() {
    assert_eq!(parse_clipboard_link("rift:ABC123"), Some("ABC123".into()));
    assert_eq!(
        parse_clipboard_link("rift:my-vanity-slug"),
        Some("my-vanity-slug".into())
    );
}

#[test]
fn clipboard_invalid() {
    assert_eq!(parse_clipboard_link("rift:"), None);
    assert_eq!(parse_clipboard_link(""), None);
    assert_eq!(parse_clipboard_link("just-some-text"), None);
    assert_eq!(parse_clipboard_link("https://"), None);
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
