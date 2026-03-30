use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Database Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkKeyDoc {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub key_hash: String,
    pub key_prefix: String,
    pub domain: String,
    pub revoked: bool,
    pub created_at: DateTime,
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSdkKeyRequest {
    /// Domain to bind this SDK key to (must be verified and owned by tenant).
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateSdkKeyResponse {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    /// The full SDK key. Shown only once at creation time.
    #[schema(example = "pk_live_a1b2c3d4e5f6g7h8i9j0")]
    pub key: String,
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SdkKeyDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "pk_live_a1b2")]
    pub key_prefix: String,
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSdkKeysResponse {
    pub keys: Vec<SdkKeyDetail>,
}
