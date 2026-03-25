/// Extracts link_id from a clipboard URL.
/// The landing page copies the full URL to clipboard (e.g. "https://go.example.com/my-link"
/// or "https://api.riftl.ink/r/ABC123"). This extracts the link_id from the path.
/// Also supports the legacy "rift:<link_id>" format for backwards compatibility.
pub fn parse_clipboard_link(text: &str) -> Option<String> {
    let text = text.trim();

    // Legacy format: "rift:<link_id>"
    if let Some(id) = text.strip_prefix("rift:") {
        let id = id.trim().to_string();
        return if id.is_empty() { None } else { Some(id) };
    }

    // URL format: extract the last path segment.
    // Handles both "https://go.example.com/my-link" and "https://api.riftl.ink/r/ABC123"
    if text.starts_with("https://") || text.starts_with("http://") {
        let path = text
            .split("//")
            .nth(1)?
            .split('?')
            .next()?
            .split('#')
            .next()?;
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        // For /r/ABC123, take the last segment. For /my-link, also the last segment.
        let link_id = segments.last()?.trim().to_string();
        return if link_id.is_empty() {
            None
        } else {
            Some(link_id)
        };
    }

    None
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
}
