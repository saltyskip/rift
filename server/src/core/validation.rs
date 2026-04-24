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

const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous",
    "ignore above",
    "you are",
    "system prompt",
    "disregard",
    "forget your instructions",
    "new instructions",
];

const VALID_ACTIONS: &[&str] = &[
    "purchase",
    "subscribe",
    "signup",
    "download",
    "read",
    "book",
    "open",
];

fn contains_injection(s: &str) -> bool {
    let lower = s.to_lowercase();
    INJECTION_PATTERNS.iter().any(|p| lower.contains(p))
}

pub fn validate_agent_action(s: &str) -> Result<(), String> {
    if !VALID_ACTIONS.contains(&s) {
        Err(format!(
            "Invalid action '{}'. Must be one of: {}",
            s,
            VALID_ACTIONS.join(", ")
        ))
    } else {
        Ok(())
    }
}

pub fn validate_cta(s: &str) -> Result<(), String> {
    if s.len() > 120 {
        return Err("CTA must be 120 characters or fewer".into());
    }
    if contains_injection(s) {
        return Err("CTA contains disallowed content".into());
    }
    Ok(())
}

pub fn validate_agent_description(s: &str) -> Result<(), String> {
    if s.len() > 500 {
        return Err("Description must be 500 characters or fewer".into());
    }
    if contains_injection(s) {
        return Err("Description contains disallowed content".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
