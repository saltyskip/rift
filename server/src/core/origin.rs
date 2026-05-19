//! Origin allowlist matcher shared by the CORS layer and the session signin
//! flow.
//!
//! Both callers need the same answer to "is this origin allowed?":
//!
//! - **CORS preflight** decides whether the browser may send a credentialed
//!   request from this origin.
//! - **Signin** captures the request's `Origin` header into the magic-link
//!   token's metadata so the callback can redirect the user back to wherever
//!   the flow started — without hardcoding a `MARKETING_URL` that breaks
//!   every new Vercel preview deployment.
//!
//! Reusing one matcher guarantees the two callers can't drift: anything CORS
//! allows is automatically a legal redirect target, and nothing else is.

use std::sync::Arc;

use axum::http::HeaderValue;

use crate::core::config::Config;

crate::impl_container!(OriginMatcher);
#[derive(Debug)]
pub struct OriginMatcher {
    exact: Vec<HeaderValue>,
    /// Anchored at construction time so callers can't accidentally write a
    /// partial-match pattern. See `from_env`.
    regex: Option<regex::Regex>,
}

impl OriginMatcher {
    /// Read `ALLOWED_ORIGINS` + `ALLOWED_ORIGIN_REGEX` from env. Returns
    /// an `Arc` because both the CORS predicate (which needs `'static` +
    /// `Send + Sync` closures) and `AppState` need shared ownership.
    pub fn from_env(cfg: &Config) -> Arc<Self> {
        let mut exact: Vec<HeaderValue> = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        if exact.is_empty() {
            for s in [
                cfg.marketing_url.as_str(),
                "http://localhost:3000",
                "http://localhost:3001",
            ] {
                if let Ok(v) = HeaderValue::from_str(s) {
                    if !exact.contains(&v) {
                        exact.push(v);
                    }
                }
            }
        }

        let regex = std::env::var("ALLOWED_ORIGIN_REGEX")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .and_then(|raw| {
                // Force full-match anchoring so a partial pattern (e.g.
                // missing `^...$`) can't let trailing junk through.
                let anchored = format!("^(?:{})$", raw.trim());
                match regex::Regex::new(&anchored) {
                    Ok(re) => Some(re),
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            pattern = %raw,
                            "invalid ALLOWED_ORIGIN_REGEX — ignored"
                        );
                        None
                    }
                }
            });

        Arc::new(Self { exact, regex })
    }

    /// CORS layer entry point — works with the `HeaderValue` shape that
    /// `AllowOrigin::predicate` hands us.
    pub fn matches(&self, origin: &HeaderValue) -> bool {
        if self.exact.iter().any(|e| e == origin) {
            return true;
        }
        let Some(re) = self.regex.as_ref() else {
            return false;
        };
        let Ok(origin_str) = origin.to_str() else {
            return false;
        };
        re.is_match(origin_str)
    }

    /// Signin/callback entry point — the origin lives as a `String` in
    /// token metadata; this avoids forcing callers to round-trip through
    /// `HeaderValue` parsing.
    pub fn matches_str(&self, origin: &str) -> bool {
        let Ok(v) = HeaderValue::from_str(origin) else {
            return false;
        };
        self.matches(&v)
    }
}

#[cfg(test)]
#[path = "origin_tests.rs"]
mod tests;
