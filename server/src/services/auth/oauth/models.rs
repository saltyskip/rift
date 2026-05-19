//! Data types for `services/auth/oauth/` — provider enum, errors, config
//! holder, and service return shapes.

use mongodb::bson::oid::ObjectId;
use std::fmt;

// ── Provider ──

/// Which upstream the user is signing in through. Only two for v1; new
/// providers (Apple, Microsoft) become new variants plus a new
/// `OauthProviderClient` impl. Email is still the identity — the variant only
/// changes how we *arrive at* the verified email.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OauthProvider {
    Github,
    Google,
}

impl OauthProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Google => "google",
        }
    }

    pub fn from_path_segment(s: &str) -> Option<Self> {
        match s {
            "github" => Some(Self::Github),
            "google" => Some(Self::Google),
            _ => None,
        }
    }
}

impl fmt::Display for OauthProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Service config ──

/// Per-provider OAuth credentials + the API base URL used to build the
/// redirect_uri the provider posts back to. Empty `client_id` or
/// `client_secret` => provider is considered disabled.
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl ProviderCredentials {
    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct OauthConfig {
    pub github: ProviderCredentials,
    pub google: ProviderCredentials,
    /// Base URL the provider redirects back to (e.g. `https://api.riftl.ink`).
    /// We append `/v1/auth/oauth/{provider}/callback` to form the
    /// `redirect_uri` for each leg.
    pub api_base_url: String,
}

// ── Provider exchange result ──

/// What a provider exchange yields. Email is the identity — we deliberately
/// don't capture the provider's stable user id in v1 (strict email-as-identity
/// per the design decision; revisit if we ever need cross-provider linking).
#[derive(Debug, Clone)]
pub struct OauthUserInfo {
    pub email: String,
}

// ── Service return shapes ──

/// Returned from `OauthService::start` — what the route handler needs to
/// redirect the browser to the provider.
#[derive(Debug, Clone)]
pub struct OauthStartOutcome {
    /// Full provider authorize URL, ready to `Location`-redirect to.
    pub authorize_url: String,
}

/// Returned from `OauthService::consume_callback` — the resolved identity
/// plus the redirect base captured at start time.
///
/// The route handler then calls `SessionsService::issue_session` and sets
/// the session cookie, exactly like the magic-link callback.
#[derive(Debug, Clone)]
pub struct OauthCallbackOutcome {
    pub user_id: ObjectId,
    pub tenant_id: ObjectId,
    /// Sanitized same-origin path on `origin` (e.g. `/account` or
    /// `/account?from=oauth`). Already passed `sanitize_next` at start time.
    pub next: String,
    /// Origin from the state token's metadata. Callers MUST re-validate this
    /// against the current `OriginMatcher` before using it — mirrors the
    /// magic-link callback's defense-in-depth pattern.
    pub origin: Option<String>,
}

// ── Errors ──

/// Errors surfaced from `OauthService` to the route handler. Each maps to a
/// `?error=<code>` query string on the `/signin` redirect — see the route
/// handler for the mapping.
#[derive(Debug)]
pub enum OauthError {
    /// Provider client_id/client_secret not set in config — `/start` returns
    /// 503; `/callback` should never reach this (state token wouldn't exist).
    NotConfigured,
    /// State token invalid, expired, or replayed.
    InvalidState,
    /// State token's `provider` field doesn't match the URL `{provider}`.
    /// Defends against an attacker swapping providers mid-flow after stealing
    /// a state token.
    ProviderMismatch,
    /// Provider's token endpoint or userinfo call failed (network, 4xx, 5xx,
    /// malformed body). Logged with details; surfaced as a generic toast.
    ProviderError(String),
    /// Provider returned an email but it wasn't verified.
    EmailUnverified,
    /// Provider returned no usable email at all (rare: misconfigured GitHub
    /// account with all-private emails and no primary).
    NoEmail,
    /// Underlying DB / repo error during user resolution or session mint.
    Internal(String),
}

impl fmt::Display for OauthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "OAuth provider not configured"),
            Self::InvalidState => write!(f, "OAuth sign-in link is invalid or has expired"),
            Self::ProviderMismatch => write!(f, "OAuth provider mismatch"),
            Self::ProviderError(e) => write!(f, "OAuth provider error: {e}"),
            Self::EmailUnverified => write!(f, "Email from provider is not verified"),
            Self::NoEmail => write!(f, "Provider did not return an email"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl OauthError {
    /// Stable error code mapped to `/signin?error=<code>` for frontend toasts.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "oauth_not_configured",
            Self::InvalidState | Self::ProviderMismatch => "oauth_state_invalid",
            Self::ProviderError(_) => "oauth_provider_error",
            Self::EmailUnverified => "oauth_email_unverified",
            Self::NoEmail => "oauth_no_email",
            Self::Internal(_) => "oauth_internal",
        }
    }
}
