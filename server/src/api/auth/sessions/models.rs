//! Request / response DTOs for `api/auth/sessions/routes.rs`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    #[schema(example = "alice@example.com")]
    pub email: String,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub user: UserSummary,
    pub tenant: TenantSummary,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: String,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantSummary {
    pub id: String,
}

/// `POST /v1/auth/secret-keys/issue` request body.
///
/// Empty in Phase 1 — the session middleware already proves the caller's
/// identity. Phase 3 will add an optional `name` field for friendly key labels;
/// keeping the body a struct (vs. `()`) makes that addition non-breaking.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct IssueKeyRequest {}
