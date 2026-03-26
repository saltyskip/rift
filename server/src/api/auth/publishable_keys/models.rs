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
    pub domain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateSdkKeyResponse {
    pub id: String,
    /// The full SDK key. Shown only once at creation time.
    pub key: String,
    pub domain: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SdkKeyDetail {
    pub id: String,
    pub key_prefix: String,
    pub domain: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSdkKeysResponse {
    pub keys: Vec<SdkKeyDetail>,
}
