//! Visitor OS detection for link resolution.
//!
//! Lives in `core` (not `api/links`) so the service layer (`resolve_alternate`)
//! can use it without a service→api backwards dependency — both transports and
//! the service share one detector.

use axum::http::HeaderMap;

// `Os` is a pub data type → it lives in `core::models` (the pub-types-in-models
// rule); re-exported here so callers keep using `core::platform::Os`.
pub use crate::core::models::Os;

/// Detect the visitor OS, preferring the `Sec-CH-UA-Platform` client hint
/// (sent by Chromium and Safari 16.4+) and falling back to User-Agent parsing
/// (older Safari/Firefox, privacy clients that strip client hints).
pub fn detect_os(headers: &HeaderMap) -> Os {
    if let Some(hint) = headers
        .get("sec-ch-ua-platform")
        .and_then(|v| v.to_str().ok())
    {
        // The header value is a quoted string token, e.g. `"macOS"`.
        let h = hint.trim().trim_matches('"').to_ascii_lowercase();
        match h.as_str() {
            "ios" => return Os::Ios,
            "android" => return Os::Android,
            "macos" => return Os::Mac,
            "windows" => return Os::Windows,
            // "linux", "chrome os", "chromium os", "unknown", "" → fall through
            // to UA parsing (desktop web + ambiguous cases).
            _ => {}
        }
    }

    match headers.get("user-agent").and_then(|v| v.to_str().ok()) {
        Some(ua) => detect_os_from_ua(ua),
        None => Os::Other,
    }
}

/// UA-only OS detection. Order matters: mobile checks come first because mobile
/// UAs also contain desktop-looking tokens (iOS UAs say "like Mac OS X";
/// Android UAs say "Linux").
pub fn detect_os_from_ua(user_agent: &str) -> Os {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        Os::Ios
    } else if ua.contains("android") {
        Os::Android
    } else if ua.contains("windows") {
        Os::Windows
    } else if ua.contains("macintosh") || ua.contains("mac os x") {
        Os::Mac
    } else {
        Os::Other
    }
}

#[cfg(test)]
#[path = "platform_tests.rs"]
mod tests;
