//! Request / response DTOs for `api/auth/sessions/routes.rs`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    #[schema(example = "alice@example.com")]
    pub email: String,
    /// Optional same-origin path to redirect to after sign-in. Validated
    /// against the request's `Origin` (or `marketing_url`) at signin time and
    /// stored in the token metadata so the callback can use it even when the
    /// query-string `next` is missing or tampered with.
    #[serde(default)]
    #[schema(example = "/checkout?tier=pro")]
    pub next: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignInResponse {
    /// Always `"sent"`. Returned even on rate-limit / invalid email so callers
    /// can't enumerate registered addresses.
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub token: String,
    #[serde(default)]
    pub next: Option<String>,
}

/// Form body for `POST /v1/auth/callback` — submitted by the interstitial
/// HTML page that `GET /v1/auth/callback` renders. The GET handler doesn't
/// consume the token (email link scanners like Avanan, Defender Safe Links,
/// and ProofPoint pre-fetch links and would burn it before the user clicks);
/// only this POST does. Scanners follow GETs but don't submit forms.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CallbackForm {
    pub token: String,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub user: UserSummary,
    pub tenant: TenantSummary,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: crate::core::public_id::UserId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantSummary {
    pub id: crate::core::public_id::TenantId,
}

/// `POST /v1/auth/secret-keys/issue` request body.
///
/// Empty in Phase 1 — the session middleware already proves the caller's
/// identity. Phase 3 will add an optional `name` field for friendly key labels;
/// keeping the body a struct (vs. `()`) makes that addition non-breaking.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct IssueKeyRequest {}

/// Query for `GET /v1/auth/cli/start`. The CLI (which only knows the API base
/// URL) opens this; the server validates the loopback `redirect_uri` and
/// bounces the browser to the dashboard's `/cli/authorize` page, which owns the
/// sign-in UI (magic-link + OAuth) and the approval click.
#[derive(Debug, Deserialize)]
pub struct CliStartQuery {
    /// The CLI's loopback listener, e.g. `http://127.0.0.1:53127/`. Must be a
    /// loopback HTTP address — the only place we'll deliver a session token.
    pub redirect_uri: String,
    /// Opaque CLI-generated nonce, passed through to the dashboard and echoed
    /// back into the loopback redirect so the CLI can bind the response to its
    /// request. Not interpreted server-side.
    #[serde(default)]
    pub state: Option<String>,
}

/// Body for `POST /v1/auth/cli/authorize`. The dashboard `/cli/authorize` page
/// posts this (with the session cookie) after the user approves. The server
/// mints a fresh CLI session for the resolved identity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CliAuthorizeRequest {
    #[schema(example = "http://127.0.0.1:53127/")]
    pub redirect_uri: String,
}

/// Response for `POST /v1/auth/cli/authorize` — the freshly minted raw session
/// token. The dashboard JS navigates the browser to the loopback `redirect_uri`
/// carrying this token, where the CLI's listener captures it.
#[derive(Debug, Serialize, ToSchema)]
pub struct CliAuthorizeResponse {
    pub token: String,
}
