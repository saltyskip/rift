use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Database Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    /// Fully qualified domain name (e.g. "go.tablefour.com").
    pub domain: String,
    pub verified: bool,
    pub verification_token: String,
    pub created_at: DateTime,
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDomainRequest {
    /// Custom domain to register (e.g. "go.tablefour.com").
    pub domain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDomainResponse {
    pub domain: String,
    pub verified: bool,
    pub verification_token: String,
    /// TXT record name to create for verification.
    pub txt_record: String,
    /// CNAME target for routing traffic.
    pub cname_target: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DomainDetail {
    pub domain: String,
    pub verified: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyDomainResponse {
    pub domain: String,
    pub verified: bool,
}
