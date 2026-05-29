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
