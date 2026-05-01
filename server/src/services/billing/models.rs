use mongodb::bson;
use serde::Deserialize;

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

// ── Plan limits ──

/// Per-tier quantitative limits. `None` means unlimited.
#[derive(Debug, Clone, Copy)]
pub struct PlanLimits {
    pub max_links: Option<u64>,
    pub max_events_per_month: Option<u64>,
    pub max_domains: Option<u64>,
    pub max_team_members: Option<u64>,
    pub max_webhooks: Option<u64>,
    pub max_affiliates: Option<u64>,
    /// Retention bucket string written into event metaField for partial TTL.
    pub retention_bucket: &'static str,
}

// ── Repo documents ──

/// Per-tenant-per-month event counter row used by `EventCountersRepo`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventCounterDoc {
    #[serde(rename = "_id")]
    pub id: String,
    pub tenant_id: mongodb::bson::oid::ObjectId,
    pub period: String, // e.g. "2026-04"
    pub count: i64,
    pub created_at: bson::DateTime,
}

/// Idempotency row used by `StripeWebhookDedupRepo`. The `_id` is the Stripe
/// `event.id`; a duplicate insert means the event has already been processed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StripeDedupDoc {
    #[serde(rename = "_id")]
    pub event_id: String,
    pub inserted_at: bson::DateTime,
}

// ── Handoff (magic-link → Stripe) ──

/// What flow the caller wants the magic link to gate. Maps onto `TokenPurpose`
/// at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingIntent {
    Subscribe,
    Portal,
}

/// Paid tier selector. `PlanTier` minus Free (Free doesn't need Stripe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingTier {
    Pro,
    Business,
    Scale,
}

impl BillingTier {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pro" => Some(Self::Pro),
            "business" => Some(Self::Business),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pro => "pro",
            Self::Business => "business",
            Self::Scale => "scale",
        }
    }

    pub fn to_plan_tier(self) -> PlanTier {
        match self {
            Self::Pro => PlanTier::Pro,
            Self::Business => PlanTier::Business,
            Self::Scale => PlanTier::Scale,
        }
    }
}

#[derive(Debug)]
pub enum BillingHandoffError {
    Invalid { code: &'static str, message: String },
    RateLimited,
}

/// Successful redeem outcome. Routes convert each variant to a 302.
#[derive(Debug)]
pub enum HandoffOutcome {
    CheckoutUrl(String),
    PortalUrl(String),
    /// Portal intent but the email doesn't resolve to a Stripe customer.
    NoSubscription,
    /// Token not found, expired, already consumed, or of an unexpected purpose.
    Expired,
}

/// Config slice passed at construction. Kept separate from `AppState` so the
/// service is easy to test with arbitrary Stripe/Resend stubs.
#[derive(Clone)]
pub struct BillingHandoffConfig {
    pub resend_api_key: String,
    pub resend_from_email: String,
    /// API domain (e.g. `https://api.riftl.ink`). Used when building the
    /// magic-link URL that embeds in the email — must hit an API route.
    pub public_url: String,
    /// Marketing site (e.g. `https://riftl.ink`). Used for every redirect
    /// target that lands on a marketing page: Stripe success/cancel, portal
    /// return, link-expired banner, no-subscription banner.
    pub marketing_url: String,
    pub stripe: StripeConfig,
}

// ── Stripe client ──

#[derive(Debug, Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub price_id_pro: String,
    pub price_id_business: String,
    pub price_id_scale: String,
    pub success_url: String,
    pub cancel_url: String,
}

impl StripeConfig {
    pub fn is_configured(&self) -> bool {
        !self.secret_key.is_empty()
    }

    pub fn price_id_for(&self, tier: PlanTier) -> Option<&str> {
        let id = match tier {
            PlanTier::Free => return None,
            PlanTier::Pro => &self.price_id_pro,
            PlanTier::Business => &self.price_id_business,
            PlanTier::Scale => &self.price_id_scale,
        };
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    }
}

#[derive(Debug)]
pub enum StripeError {
    NotConfigured,
    MissingPriceId(PlanTier),
    Api(String),
    Network(String),
}

impl std::fmt::Display for StripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Stripe not configured"),
            Self::MissingPriceId(t) => write!(f, "Missing Stripe price ID for tier {t:?}"),
            Self::Api(e) => write!(f, "Stripe API error: {e}"),
            Self::Network(e) => write!(f, "Network error calling Stripe: {e}"),
        }
    }
}

/// Result of creating a Checkout session — the caller redirects the user
/// to `url`.
#[derive(Debug, serde::Deserialize)]
pub struct CheckoutSession {
    pub url: String,
}

/// Options for the billing-handoff Checkout session. Unlike the Bearer-authed
/// `create_checkout_session`, this variant doesn't assume an existing tenant —
/// either `customer_id` is known (tenant upgrading) or `customer_email` +
/// `pending_email` are set (brand-new customer, the webhook will materialize
/// the tenant on payment completion).
pub struct HandoffCheckoutOpts<'a> {
    pub tier: PlanTier,
    pub customer_id: Option<&'a str>,
    pub customer_email: Option<&'a str>,
    /// Written to subscription metadata so the webhook handler can create a
    /// tenant after payment without any client-side state.
    pub pending_email: Option<&'a str>,
    /// When set, echoed back via subscription metadata and
    /// `client_reference_id` so the webhook can find the tenant fast.
    pub tenant_id_hex: Option<&'a str>,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
}

/// Result of creating a Billing Portal session — the caller redirects the
/// user to `url`.
#[derive(Debug, serde::Deserialize)]
pub struct PortalSession {
    pub url: String,
}

#[derive(Debug)]
pub enum WebhookVerifyError {
    /// Signature header missing or malformed.
    BadHeader,
    /// Timestamp older than the tolerance window.
    TimestampTooOld,
    /// No `v1=` component in the header matched the expected signature.
    SignatureMismatch,
}

impl std::fmt::Display for WebhookVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadHeader => write!(f, "malformed Stripe-Signature header"),
            Self::TimestampTooOld => write!(f, "webhook timestamp older than 5 min"),
            Self::SignatureMismatch => write!(f, "signature mismatch"),
        }
    }
}
