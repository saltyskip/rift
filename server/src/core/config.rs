/// Server configuration resolved from environment variables with defaults.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,

    // ── MongoDB ──
    pub mongo_uri: String,
    pub mongo_db: String,

    // ── Resend (email) ──
    pub resend_api_key: String,
    pub resend_from_email: String,

    // ── Auth / rate limits ──
    /// Public-facing URL of *this* API server. Used to build links that must
    /// hit an API route (email-verify URL, magic-link redeem URL). In prod
    /// this is `https://api.riftl.ink`.
    pub public_url: String,
    /// Public-facing URL of the *marketing* site. Used to build redirect
    /// targets that land on a marketing page (Stripe success/cancel,
    /// link-expired banner, billing-portal return). In prod this is
    /// `https://riftl.ink`. Falls back to `public_url` for dev so a solo
    /// server still works end-to-end.
    pub marketing_url: String,
    /// `Domain` attribute for the session cookie. Derived from `MARKETING_URL`'s
    /// host so each environment scopes its cookies to its own subtree:
    /// prod (`riftl.ink`) → `.riftl.ink`, sandbox (`sandbox.riftl.ink`) →
    /// `.sandbox.riftl.ink`, local dev (`localhost`) → `None`. This stops a
    /// sandbox session cookie from leaking into prod. `COOKIE_DOMAIN` can
    /// override for unusual deployments.
    pub cookie_domain: Option<String>,
    /// Whether the session cookie gets `Secure`. Derived from `MARKETING_URL`
    /// scheme — `https://` → true, otherwise false. Browsers refuse `Secure`
    /// cookies over plain HTTP, so dev/localhost MUST be false. When
    /// `cookie_same_site == None`, this is forced to `true` because
    /// browsers reject `SameSite=None` without `Secure`.
    pub cookie_secure: bool,
    /// `SameSite` attribute for the session cookie. `Lax` (the default)
    /// blocks cross-origin subresource requests from sending the cookie —
    /// good CSRF posture for prod where marketing + API share a parent
    /// domain. `None` allows the cookie on every cross-origin request and
    /// is intended for preview/sandbox environments where the marketing
    /// site (e.g. a `*.vercel.app` preview URL) and the API live on
    /// different parent domains. `None` widens the CSRF surface — keep
    /// it confined to non-prod via `COOKIE_SAMESITE`.
    pub cookie_same_site: CookieSameSite,
    pub free_daily_limit: i64,

    // ── Sentry ──
    pub sentry_dsn: String,
    /// Deployment environment tag for Sentry (e.g. `production`, `staging`,
    /// `development`). Defaults to `development` so misconfigured prod
    /// surfaces obviously rather than silently labeling local events.
    pub environment: String,
    /// Short git SHA of the running build, sourced from the `GIT_SHA` env
    /// var (set by the Docker build-arg in `Dockerfile`). Used as the Sentry
    /// release tag. Falls back to `unknown` when the build wasn't tagged.
    pub git_sha: String,

    // ── x402 ──
    pub x402_facilitator_url: String,
    pub x402_recipient: String,
    pub x402_price_display: String,
    pub x402_enabled: bool,
    pub cdp_api_key_id: String,
    pub cdp_api_key_secret: String,

    // ── x402 resource description (shown to payers) ──
    pub x402_description: String,

    // ── Custom domains ──
    pub primary_domain: String,

    // ── Fly.io (cert provisioning) ──
    pub fly_api_token: String,
    pub fly_app_name: String,

    // ── Stripe (Plan A billing) ──
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub stripe_price_id_pro: String,
    pub stripe_price_id_business: String,
    pub stripe_price_id_scale: String,
    pub stripe_success_url: String,
    pub stripe_cancel_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),

            mongo_uri: std::env::var("MONGO_URI").unwrap_or_default(),
            mongo_db: std::env::var("MONGO_DB").unwrap_or_else(|_| "relay".to_string()),

            resend_api_key: std::env::var("RESEND_API_KEY").unwrap_or_default(),
            resend_from_email: std::env::var("RESEND_FROM_EMAIL")
                .unwrap_or_else(|_| "Rift <noreply@updates.riftl.ink>".to_string()),

            public_url: std::env::var("PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            // MARKETING_URL is separate because PUBLIC_URL points at the API
            // domain in prod. Fall back to PUBLIC_URL for local dev.
            marketing_url: std::env::var("MARKETING_URL").unwrap_or_else(|_| {
                std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
            }),
            cookie_domain: resolve_cookie_domain(
                std::env::var("COOKIE_DOMAIN").ok().as_deref(),
                std::env::var("MARKETING_URL")
                    .ok()
                    .as_deref()
                    .or(std::env::var("PUBLIC_URL").ok().as_deref())
                    .unwrap_or("http://localhost:3000"),
            ),
            cookie_secure: {
                let same_site =
                    CookieSameSite::from_env_str(std::env::var("COOKIE_SAMESITE").ok().as_deref());
                let scheme_secure = std::env::var("MARKETING_URL")
                    .ok()
                    .as_deref()
                    .or(std::env::var("PUBLIC_URL").ok().as_deref())
                    .unwrap_or("http://localhost:3000")
                    .starts_with("https://");
                // SameSite=None requires Secure or browsers drop the cookie.
                scheme_secure || same_site == CookieSameSite::None
            },
            cookie_same_site: CookieSameSite::from_env_str(
                std::env::var("COOKIE_SAMESITE").ok().as_deref(),
            ),
            free_daily_limit: std::env::var("FREE_DAILY_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),

            sentry_dsn: std::env::var("SENTRY_DSN").unwrap_or_default(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            git_sha: std::env::var("GIT_SHA").unwrap_or_else(|_| "unknown".to_string()),

            x402_facilitator_url: std::env::var("X402_FACILITATOR_URL")
                .unwrap_or_else(|_| "https://facilitator.x402.org".to_string()),
            x402_recipient: std::env::var("X402_RECIPIENT").unwrap_or_default(),
            x402_price_display: std::env::var("X402_PRICE").unwrap_or_else(|_| "0.01".to_string()),
            x402_enabled: std::env::var("X402_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            cdp_api_key_id: std::env::var("CDP_API_KEY_ID").unwrap_or_default(),
            cdp_api_key_secret: std::env::var("CDP_API_KEY_SECRET").unwrap_or_default(),

            x402_description: std::env::var("X402_DESCRIPTION")
                .unwrap_or_else(|_| "Rift API request".to_string()),

            primary_domain: std::env::var("PRIMARY_DOMAIN")
                .unwrap_or_else(|_| "riftl.ink".to_string()),

            fly_api_token: std::env::var("FLY_API_TOKEN").unwrap_or_default(),
            fly_app_name: std::env::var("FLY_APP_NAME").unwrap_or_default(),

            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            stripe_price_id_pro: std::env::var("STRIPE_PRICE_ID_PRO").unwrap_or_default(),
            stripe_price_id_business: std::env::var("STRIPE_PRICE_ID_BUSINESS").unwrap_or_default(),
            stripe_price_id_scale: std::env::var("STRIPE_PRICE_ID_SCALE").unwrap_or_default(),
            stripe_success_url: std::env::var("STRIPE_SUCCESS_URL").unwrap_or_default(),
            stripe_cancel_url: std::env::var("STRIPE_CANCEL_URL").unwrap_or_default(),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// `SameSite` attribute for the session cookie. Parsed from the
/// `COOKIE_SAMESITE` env var; unknown values fall back to `Lax`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

impl CookieSameSite {
    pub fn from_env_str(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("strict") => Self::Strict,
            Some("none") => Self::None,
            // Default + explicit "lax" + anything unrecognised → Lax.
            // Unrecognised values fall back rather than panic so a typo
            // doesn't kill the server at startup.
            _ => Self::Lax,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "Strict",
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }
}

/// Resolve the cookie `Domain` attribute. Explicit `COOKIE_DOMAIN` wins
/// (normalised to a leading dot). Otherwise derive from the marketing URL's
/// host; return `None` for `localhost` or numeric IPs so dev works without
/// the browser silently dropping the cookie.
///
/// Why the leading dot: RFC 6265 treats `Domain=.example.com` and
/// `Domain=example.com` identically in modern browsers, but the leading
/// dot is the conventional safe form and makes the intent ("send to this
/// host and its subdomains") explicit at the call site.
fn resolve_cookie_domain(override_env: Option<&str>, marketing_url: &str) -> Option<String> {
    if let Some(raw) = override_env {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(if trimmed.starts_with('.') {
            trimmed.to_string()
        } else {
            format!(".{trimmed}")
        });
    }

    let parsed = url::Url::parse(marketing_url).ok()?;
    let host = parsed.host_str()?;
    // IPv6 hosts come back bracketed (`[::1]`); strip before IP-parsing.
    let bare = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host);
    if bare == "localhost" || bare.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    Some(format!(".{host}"))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
