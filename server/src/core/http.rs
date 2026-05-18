//! Generic HTTP helpers shared across route handlers.

use axum::http::HeaderMap;

/// Extract the caller's IP for rate-limit / logging purposes. Reads
/// `X-Forwarded-For` (first hop) because Fly's proxy sets it; falls back to
/// `"local"` when missing, which gives dev/local callers a single shared
/// bucket rather than no bucket at all.
///
/// Middleware that already holds a `Request` can use the variant in
/// `api/auth/middleware.rs::client_ip` which additionally falls back to
/// `ConnectInfo<SocketAddr>` when the header is missing.
pub fn client_ip_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "local".to_string())
}
