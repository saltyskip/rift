/// Normalize a user-entered URL by prepending `https://` if no scheme is present.
pub fn normalize_web_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}
