/// Extracts link_id from clipboard text with format "rift:<link_id>".
/// Used on iOS where the landing page copies "rift:<link_id>" to clipboard.
pub fn parse_clipboard_link(text: &str) -> Option<String> {
    text.strip_prefix("rift:")
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
}

/// Extracts link_id from Android install referrer string.
/// The referrer contains "rift_link=<link_id>" as a query parameter.
pub fn parse_referrer_link(referrer: &str) -> Option<String> {
    referrer
        .split('&')
        .find_map(|pair| {
            pair.strip_prefix("rift_link=")
                .map(|v| v.trim().to_string())
        })
        .filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_valid() {
        assert_eq!(parse_clipboard_link("rift:ABC123"), Some("ABC123".into()));
        assert_eq!(
            parse_clipboard_link("rift:my-vanity-slug"),
            Some("my-vanity-slug".into())
        );
    }

    #[test]
    fn clipboard_invalid() {
        assert_eq!(parse_clipboard_link("rift:"), None);
        assert_eq!(parse_clipboard_link("other:ABC123"), None);
        assert_eq!(parse_clipboard_link(""), None);
        assert_eq!(parse_clipboard_link("ABC123"), None);
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
}
