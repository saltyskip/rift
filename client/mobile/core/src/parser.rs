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
#[path = "parser_tests.rs"]
mod tests;
