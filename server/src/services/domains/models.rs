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
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDomainResponse {
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = false)]
    pub verified: bool,
    #[schema(example = "relay-verify-a1b2c3d4e5f6")]
    pub verification_token: String,
    /// TXT record name to create for verification.
    #[schema(example = "_relay-challenge.go.tablefour.com")]
    pub txt_record: String,
    /// CNAME target for routing traffic.
    #[schema(example = "cname.riftl.ink")]
    pub cname_target: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DomainDetail {
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = true)]
    pub verified: bool,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyDomainResponse {
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = true)]
    pub verified: bool,
}
