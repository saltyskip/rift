use std::net::IpAddr;
use url::Url;

const MAX_URL_LEN: usize = 2048;

const BLOCKED_SCHEMES: &[&str] = &["javascript", "data", "vbscript", "blob", "file"];

const STORE_HOSTS: &[&str] = &["apps.apple.com", "itunes.apple.com", "play.google.com"];

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                // CGNAT range 100.64.0.0/10
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
                // AWS metadata endpoint
                || (v4.octets()[0] == 169 && v4.octets()[1] == 254)
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

fn check_host_not_private(parsed: &Url) -> Result<(), String> {
    let Some(host) = parsed.host_str() else {
        return Err("URL must have a host".into());
    };
    if host == "localhost" || host == "127.0.0.1" || host == "[::1]" {
        return Err("URLs pointing to localhost are not allowed".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err("URLs pointing to private/reserved IP ranges are not allowed".into());
        }
    }
    Ok(())
}

/// Validate a web-facing URL (web_url). Must be http or https, no private IPs.
pub fn validate_web_url(s: &str) -> Result<(), String> {
    if s.len() > MAX_URL_LEN {
        return Err(format!("URL exceeds maximum length of {MAX_URL_LEN}"));
    }
    let parsed = Url::parse(s).map_err(|e| format!("Invalid URL: {e}"))?;
    let scheme = parsed.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!("URL scheme must be http or https, got '{scheme}'"));
    }
    check_host_not_private(&parsed)
}

/// Validate a deep link URI (ios_deep_link, android_deep_link).
/// Allows custom schemes (myapp://) but blocks dangerous ones.
pub fn validate_deep_link(s: &str) -> Result<(), String> {
    if s.len() > MAX_URL_LEN {
        return Err(format!("URL exceeds maximum length of {MAX_URL_LEN}"));
    }
    let parsed = Url::parse(s).map_err(|e| format!("Invalid URL: {e}"))?;
    let scheme = parsed.scheme().to_lowercase();
    if BLOCKED_SCHEMES.contains(&scheme.as_str()) {
        return Err(format!("Scheme '{scheme}' is not allowed"));
    }
    Ok(())
}

/// Validate a store URL. Must be https pointing to Apple or Google domains.
pub fn validate_store_url(s: &str) -> Result<(), String> {
    validate_web_url(s)?;
    let parsed = Url::parse(s).expect("already validated");
    let host = parsed.host_str().unwrap_or_default();
    if !STORE_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
    {
        return Err(format!(
            "Store URL host must be an Apple or Google domain, got '{host}'"
        ));
    }
    Ok(())
}

/// Validate a hex color string (#RGB or #RRGGBB).
pub fn validate_hex_color(s: &str) -> Result<(), String> {
    let s = s.trim();
    let valid = (s.len() == 7 || s.len() == 4)
        && s.starts_with('#')
        && s[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err("Must be a hex color (#RRGGBB or #RGB)".into());
    }
    Ok(())
}

/// Validate metadata size (max 4KB serialized).
pub fn validate_metadata(v: &serde_json::Value) -> Result<(), String> {
    let serialized = serde_json::to_string(v).unwrap_or_default();
    if serialized.len() > 4096 {
        return Err("metadata must be under 4KB".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_url_valid() {
        assert!(validate_web_url("https://example.com").is_ok());
        assert!(validate_web_url("https://example.com/path?q=1").is_ok());
        assert!(validate_web_url("http://example.com").is_ok());
    }

    #[test]
    fn web_url_blocked_schemes() {
        assert!(validate_web_url("javascript:alert(1)").is_err());
        assert!(validate_web_url("data:text/html,<h1>hi</h1>").is_err());
        assert!(validate_web_url("ftp://example.com").is_err());
        assert!(validate_web_url("myapp://path").is_err());
    }

    #[test]
    fn web_url_blocked_hosts() {
        assert!(validate_web_url("http://localhost/path").is_err());
        assert!(validate_web_url("http://127.0.0.1/path").is_err());
        assert!(validate_web_url("http://10.0.0.1/path").is_err());
        assert!(validate_web_url("http://192.168.1.1/path").is_err());
        assert!(validate_web_url("http://169.254.169.254/latest").is_err());
    }

    #[test]
    fn deep_link_valid() {
        assert!(validate_deep_link("myapp://product/123").is_ok());
        assert!(validate_deep_link("https://example.com/path").is_ok());
        assert!(validate_deep_link("fb://profile/123").is_ok());
    }

    #[test]
    fn deep_link_blocked() {
        assert!(validate_deep_link("javascript:alert(1)").is_err());
        assert!(validate_deep_link("data:text/html,x").is_err());
        assert!(validate_deep_link("vbscript:msgbox").is_err());
    }

    #[test]
    fn store_url_valid() {
        assert!(validate_store_url("https://apps.apple.com/app/id123").is_ok());
        assert!(
            validate_store_url("https://play.google.com/store/apps/details?id=com.example").is_ok()
        );
    }

    #[test]
    fn store_url_wrong_host() {
        assert!(validate_store_url("https://evil.com/fake-store").is_err());
        assert!(validate_store_url("https://example.com").is_err());
    }

    #[test]
    fn hex_color_valid() {
        assert!(validate_hex_color("#ff0000").is_ok());
        assert!(validate_hex_color("#FFF").is_ok());
        assert!(validate_hex_color("#0d9488").is_ok());
    }

    #[test]
    fn hex_color_invalid() {
        assert!(validate_hex_color("red").is_err());
        assert!(validate_hex_color("#gg0000").is_err());
        assert!(validate_hex_color("ff0000").is_err());
        assert!(validate_hex_color("#ff00").is_err());
    }

    #[test]
    fn metadata_size() {
        let small = serde_json::json!({"key": "value"});
        assert!(validate_metadata(&small).is_ok());

        let big = serde_json::json!({"key": "x".repeat(5000)});
        assert!(validate_metadata(&big).is_err());
    }
}
