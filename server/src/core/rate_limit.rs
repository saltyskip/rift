use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use dashmap::DashMap;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

/// In-memory token bucket rate limiter keyed by IP.
pub struct RateLimiter {
    buckets: DashMap<String, Bucket>,
    max_tokens: u32,
    refill_rate: f64, // tokens per second
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `requests_per_minute`: sustained rate
    /// - `burst`: max tokens (allows short bursts above the sustained rate)
    pub fn new(requests_per_minute: u32, burst: u32) -> Self {
        Self {
            buckets: DashMap::new(),
            max_tokens: burst,
            refill_rate: requests_per_minute as f64 / 60.0,
        }
    }

    /// Try to consume one token for the given key. Returns true if allowed.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.max_tokens as f64,
            last_refill: now,
        });

        let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
        entry.tokens = (entry.tokens + elapsed * self.refill_rate).min(self.max_tokens as f64);
        entry.last_refill = now;

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Axum middleware that rate-limits by client IP.
pub async fn rate_limit_middleware(
    axum::extract::Extension(limiter): axum::extract::Extension<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    let ip = extract_ip(&req);

    if !limiter.check(&ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Too many requests",
                "code": "rate_limited",
                "hint": "Slow down or authenticate with an API key"
            })),
        )
            .into_response();
    }

    next.run(req).await
}

fn extract_ip(req: &Request) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            req.extensions()
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}
