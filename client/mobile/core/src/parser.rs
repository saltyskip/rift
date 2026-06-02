/// Extracts a Rift link_id from a clipboard URL, validating that the URL's
/// host is one the SDK trusts (`allowed_hosts` — the tenant's verified domains
/// plus the API host). Host validation prevents an unrelated URL left on the
/// clipboard (e.g. `https://othersite.com/promo`) from being mis-attributed as
/// a Rift deep link.
///
/// The landing page / Web SDK copies the full resolver URL to the clipboard
/// (e.g. `https://go.example.com/my-link` or `https://api.riftl.ink/r/ABC123`);
/// this returns the trailing path segment as the link_id. Host comparison is
/// case-insensitive and ignores any port.
pub fn parse_clipboard_link(text: &str, allowed_hosts: &[String]) -> Option<String> {
    let text = text.trim();
    if !(text.starts_with("https://") || text.starts_with("http://")) {
        return None;
    }

    let after_scheme = text.split("//").nth(1)?;
    // Split host from the rest at the first '/'. A bare domain is not a link.
    let (host_part, path_part) = after_scheme.split_once('/')?;
    // Strip optional userinfo ("user@") and port (":443").
    let host = host_part
        .rsplit('@')
        .next()
        .unwrap_or(host_part)
        .split(':')
        .next()
        .unwrap_or(host_part);
    if !host_is_allowed(host, allowed_hosts) {
        return None;
    }

    let path = path_part.split(['?', '#']).next().unwrap_or("");
    let link_id = path.split('/').rfind(|s| !s.is_empty())?;
    validate_link_id(link_id)
}

fn host_is_allowed(host: &str, allowed_hosts: &[String]) -> bool {
    let host = host.to_ascii_lowercase();
    allowed_hosts
        .iter()
        .any(|allowed| allowed.trim().to_ascii_lowercase() == host)
}

/// Validate an extracted segment looks like a Rift link_id — mirrors the
/// server's `is_valid_link_id`: non-empty, <= 64 chars, alphanumeric or `-`.
fn validate_link_id(id: &str) -> Option<String> {
    let id = id.trim();
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }
    Some(id.to_string())
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
