//! Request / response DTOs and query decoders for `api/auth/secret_keys/routes.rs`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Signup / Verify ──

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct VerifyQuery {
    pub token: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignupResponse {
    pub message: String,
    pub key_prefix: String,
    pub note: String,
}

// ── Secret Key CRUD ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestCreateKeyRequest {
    /// Email of a verified user on this tenant who will receive the confirmation code.
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmCreateKeyRequest {
    /// Email of the user who received the confirmation code.
    #[schema(example = "alice@example.com")]
    pub email: String,
    /// The 6-character confirmation code from the email.
    #[schema(example = "ABC123")]
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateKeyResponse {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    /// The full secret key. Shown only once at creation time.
    #[schema(example = "rl_live_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2")]
    pub key: String,
    #[schema(example = "rl_live_a1b2c3d4...")]
    pub key_prefix: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretKeyDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "rl_live_a1b2c3d4...")]
    pub key_prefix: String,
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0f")]
    pub created_by: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSecretKeysResponse {
    pub keys: Vec<SecretKeyDetail>,
}
