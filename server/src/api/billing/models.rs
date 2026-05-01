//! Request / response DTOs for `api/billing/routes.rs`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::auth::tenants::repo::{BillingMethod, PlanTier, SubscriptionStatus};

/// Slim JSON shape for the status endpoint. Rendered from `BillingStatus` so
/// the external contract stays stable if we add internal fields later.
#[derive(Serialize, ToSchema)]
pub struct BillingStatusResponse {
    pub plan_tier: PlanTier,
    pub effective_tier: PlanTier,
    pub comp_active: bool,
    pub billing_method: BillingMethod,
    pub status: SubscriptionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<i64>,
    pub limits: LimitsView,
}

#[derive(Serialize, ToSchema)]
pub struct LimitsView {
    pub max_links: Option<u64>,
    pub max_events_per_month: Option<u64>,
    pub max_domains: Option<u64>,
    pub max_team_members: Option<u64>,
    pub max_webhooks: Option<u64>,
    pub analytics_retention: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct CheckoutSessionResponse {
    pub checkout_url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MagicLinkRequest {
    /// Email to send the magic link to. Always returns 200 regardless of
    /// whether an account exists — prevents enumeration.
    #[schema(example = "you@company.com")]
    pub email: String,
    /// One of: `subscribe`, `portal`. Determines the flow on redemption.
    #[schema(example = "subscribe")]
    pub intent: String,
    /// Required when `intent` is `subscribe`. One of: `pro`, `business`, `scale`.
    #[schema(example = "pro")]
    pub tier: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MagicLinkResponse {
    /// Always `"sent"`. Never indicates whether the email actually dispatched —
    /// prevents account enumeration via timing or error signals.
    pub status: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct PortalSessionResponse {
    pub portal_url: String,
}
