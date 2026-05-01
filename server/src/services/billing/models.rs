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

// ── Quota types ──

/// Quotable resource categories. Each maps to a specific enforcement path.
/// `TrackEvent` covers both click and conversion writes — they share the
/// `max_events_per_month` limit on the pricing page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    CreateLink,
    TrackEvent,
    CreateDomain,
    InviteTeamMember,
    CreateWebhook,
    CreateAffiliate,
}

impl Resource {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CreateLink => "create_link",
            Self::TrackEvent => "track_event",
            Self::CreateDomain => "create_domain",
            Self::InviteTeamMember => "invite_team_member",
            Self::CreateWebhook => "create_webhook",
            Self::CreateAffiliate => "create_affiliate",
        }
    }
}

/// Outcome of a quota check. In Phase A-1 (log-only) we log `Exceeded` and
/// continue; Phase A-2 will return it as a `402 Payment Required` to clients.
#[derive(Debug)]
pub enum QuotaError {
    Exceeded {
        resource: Resource,
        limit: u64,
        current: u64,
    },
    Billing(BillingError),
}

impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exceeded {
                resource,
                limit,
                current,
            } => write!(
                f,
                "quota exceeded on {} ({}/{})",
                resource.code(),
                current,
                limit
            ),
            Self::Billing(e) => write!(f, "billing error: {e}"),
        }
    }
}

impl From<BillingError> for QuotaError {
    fn from(err: BillingError) -> Self {
        QuotaError::Billing(err)
    }
}

/// Whether quota checks hard-reject or just log the would-be rejection.
///
/// `LogOnly` is the safe default — every code path calls `QuotaService::check`
/// but it always returns `Ok(())`, emitting `tracing::warn!` when a tenant
/// would have been rejected. `Enforce` flips `QuotaError::Exceeded` into a
/// real error the caller maps to `402 Payment Required`.
///
/// Controlled by `QUOTA_ENFORCEMENT=enforce` (default: log_only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    LogOnly,
    Enforce,
}

impl EnforcementMode {
    pub fn from_env_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("enforce") {
            Self::Enforce
        } else {
            Self::LogOnly
        }
    }
}
