use mongodb::bson;

use crate::services::auth::tenants::repo::{BillingMethod, PlanTier, SubscriptionStatus};

/// Response shape for `GET /v1/billing/status`. Credential-agnostic — works
/// for Free / Stripe / X402 tenants identically.
#[derive(Debug, Clone)]
pub struct BillingStatus {
    pub plan_tier: PlanTier,
    pub effective_tier: PlanTier,
    pub comp_active: bool,
    pub billing_method: BillingMethod,
    pub status: SubscriptionStatus,
    pub current_period_end: Option<bson::DateTime>,
}

#[derive(Debug)]
pub enum BillingError {
    TenantNotFound,
    Internal(String),
}

impl std::fmt::Display for BillingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TenantNotFound => write!(f, "Tenant not found"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}
