//! Data types for `services/auth/tenants/` — DB document, plan/billing enums,
//! and update payloads.

use mongodb::bson;
use serde::{Deserialize, Serialize};

use crate::core::public_id::TenantId;

// ── Plan / billing enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    #[default]
    Free,
    Pro,
    Business,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingMethod {
    #[default]
    Free,
    Stripe,
    /// Reserved for Plan B (agent lane). Not written by Plan A code.
    X402,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    #[default]
    Active,
    PastDue,
    Canceled,
}

/// Whether a link auto-redirects eligible visitors to the platform destination
/// or always shows the landing page. Lives here (the foundational tenants
/// domain) so both `TenantDoc` (tenant-wide default) and `Link` (per-link
/// override) can share one type. See `core::platform` for the routing tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RedirectMode {
    /// Auto-redirect eligible visitors straight to the destination: a zero-flash
    /// 307 for desktop Tier-1 targets with a user-activation signal; the landing
    /// page (which preserves unfurls + the clipboard tap) for everything else.
    Auto,
    /// Always show the landing page; the visitor taps to continue. The safe
    /// fallback for legacy links that predate this field.
    #[default]
    Off,
}

// ── DB Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<TenantId>,
    pub monthly_quota: i64,
    pub created_at: bson::DateTime,

    #[serde(default)]
    pub plan_tier: PlanTier,
    #[serde(default)]
    pub billing_method: BillingMethod,
    #[serde(default)]
    pub status: SubscriptionStatus,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<bson::DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<bson::DateTime>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_subscription_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_tier: Option<PlanTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_until: Option<bson::DateTime>,

    /// Tenant-wide default for new links' `redirect_mode`. `None` ⇒ links
    /// default to `Auto` at create time. Per-link `redirect_mode` overrides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_redirect_mode: Option<RedirectMode>,
}

impl Default for TenantDoc {
    fn default() -> Self {
        Self {
            id: None,
            monthly_quota: 100,
            created_at: bson::DateTime::now(),
            plan_tier: PlanTier::Free,
            billing_method: BillingMethod::Free,
            status: SubscriptionStatus::Active,
            current_period_start: None,
            current_period_end: None,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            comp_tier: None,
            comp_until: None,
            default_redirect_mode: None,
        }
    }
}

/// Fields a Stripe subscription event updates atomically. Pass `None` for
/// fields the caller doesn't want to touch; `Some(value)` replaces.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionUpdate {
    pub plan_tier: Option<PlanTier>,
    pub billing_method: Option<BillingMethod>,
    pub status: Option<SubscriptionStatus>,
    pub current_period_start: Option<bson::DateTime>,
    pub current_period_end: Option<bson::DateTime>,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
}
