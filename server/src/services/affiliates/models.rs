use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AffiliateStatus {
    /// Postbacks fire and the affiliate's credentials work.
    #[default]
    Active,
    /// Affiliate is paused — credentials still authenticate but the
    /// (future) dispatcher will skip postback delivery for this affiliate.
    Disabled,
}

/// Stored affiliate record.
///
/// `postback_url` and `signing_secret` are intentionally absent in v1 —
/// they land with the postback dispatcher in a follow-up PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affiliate {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub name: String,
    pub partner_key: String,
    pub status: AffiliateStatus,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// ── DTOs ──

/// Register a partner that will drive traffic to your app. The `partner_key`
/// must be a unique lowercase slug per tenant — used to route postbacks
/// (when the dispatcher ships) and identify the partner in event metadata.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAffiliateRequest {
    /// Human-readable partner name.
    #[schema(example = "Bcom")]
    pub name: String,
    /// Unique slug per tenant. Lowercase letters, digits, and `-`. 2–32 chars.
    #[schema(example = "bcom")]
    pub partner_key: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AffiliateDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "Bcom")]
    pub name: String,
    #[schema(example = "bcom")]
    pub partner_key: String,
    pub status: AffiliateStatus,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListAffiliatesResponse {
    pub affiliates: Vec<AffiliateDetail>,
}

/// Patch an affiliate. All fields are optional; the body must contain at
/// least one to take effect. `partner_key` is intentionally immutable — it's
/// used as a stable identifier in scoped credentials and (future) postbacks.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAffiliateRequest {
    #[schema(example = "Bcom (Italy)")]
    pub name: Option<String>,
    pub status: Option<AffiliateStatus>,
}

// ── Credential DTOs ──

/// Returned ONCE on successful credential mint. The advertiser is responsible
/// for handing `api_key` to the partner out-of-band. Rift will never reveal
/// it again — list endpoints only return the prefix.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateAffiliateCredentialResponse {
    /// Credential ObjectId (the secret key id).
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    /// Affiliate this credential is scoped to.
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub affiliate_id: String,
    /// Plaintext `rl_live_…` key. Shown only once.
    #[schema(example = "rl_live_4f2c3a8b9d0e1f2a3b4c5d6e7f8a9b0c")]
    pub api_key: String,
    /// First-18-chars-of-key prefix shown in subsequent list calls.
    #[schema(example = "rl_live_4f2c3a8b9d...")]
    pub key_prefix: String,
    #[schema(example = "2026-04-25T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AffiliateCredentialDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "rl_live_4f2c3a8b9d...")]
    pub key_prefix: String,
    #[schema(example = "2026-04-25T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListAffiliateCredentialsResponse {
    pub credentials: Vec<AffiliateCredentialDetail>,
}
