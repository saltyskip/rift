use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Database Document ──

/// Domain role: Primary domains serve landing pages and resolve links.
/// Alternate domains exist solely as Universal Link trampolines — the
/// "Open in App" button on a primary domain's landing page points to
/// the alternate domain so the cross-domain tap triggers Universal Links.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DomainRole {
    #[default]
    #[serde(rename = "primary")]
    Primary,
    #[serde(rename = "alternate")]
    Alternate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    /// Fully qualified domain name (e.g. "go.tablefour.com").
    pub domain: String,
    pub verified: bool,
    pub verification_token: String,
    #[serde(default)]
    pub role: DomainRole,
    pub created_at: DateTime,
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDomainRequest {
    /// Custom domain to register (e.g. "go.tablefour.com").
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    /// Domain role: "primary" (default) or "alternate" (Universal Link trampoline).
    #[schema(example = "primary")]
    pub role: Option<String>,
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
    #[schema(example = "primary")]
    pub role: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyDomainResponse {
    #[schema(example = "go.tablefour.com")]
    pub domain: String,
    #[schema(example = true)]
    pub verified: bool,
    /// TLS certificate status: "none", "provisioning", "active", "failed", or "unknown".
    #[schema(example = "active")]
    pub tls: String,
}

// ── Errors ──

use crate::services::billing::quota::QuotaError;

#[derive(Debug)]
pub enum DomainError {
    AlreadyRegistered,
    AlternateLimit,
    QuotaExceeded(QuotaError),
    Internal(String),
}

impl From<QuotaError> for DomainError {
    fn from(err: QuotaError) -> Self {
        DomainError::QuotaExceeded(err)
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRegistered => write!(f, "Domain already registered"),
            Self::AlternateLimit => write!(f, "Only one alternate domain allowed per team"),
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}
