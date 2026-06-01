use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::public_id::{AffiliateId, TenantId, WebhookId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    /// Fired when any user or agent resolves one of your links.
    Click,
    /// Fired when a mobile SDK reports a link-touch via
    /// `POST /v1/lifecycle/attribute`. Payload includes `user_id` if the
    /// install is already identified (existing-install re-attribution
    /// path), and a snapshot of `link_metadata` at fire time so receivers
    /// can act without a follow-up Link lookup.
    Attribute,
    /// Fired when a conversion event is ingested via a source webhook.
    /// Carries a stable `event_id` so customer handlers can dedup on retries.
    Conversion,
    /// Fired when `PUT /v1/lifecycle/identify` successfully binds an
    /// install to a user. Payload carries the resolved triple
    /// `{user_id, link_id, link_metadata}` so receivers can react (grant
    /// entitlements, etc.) without a follow-up Link lookup.
    Identify,
}

/// Optional delivery filters narrowing *which* events reach a webhook,
/// on top of the `events` type subscription. Empty (all fields `None`)
/// means "no narrowing" — the webhook receives every subscribed event,
/// preserving pre-filter behaviour.
///
/// Semantics are AND across present dimensions; an absent dimension is
/// not a constraint. New dimensions (e.g. `conversion_types`) are added
/// here as further optional fields — additive, no migration, and the
/// match logic in [`WebhookFilters::matches`] extends in lock-step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WebhookFilters {
    /// Deliver only conversions whose last-touch credited link is pinned
    /// to this affiliate. Omit to receive conversions regardless of
    /// affiliate. Validated to exist in the tenant at create time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliate_id: Option<AffiliateId>,
}

impl WebhookFilters {
    /// True when no dimension is set — the webhook receives everything.
    pub fn is_empty(&self) -> bool {
        self.affiliate_id.is_none()
    }

    /// Whether an event with the given match context should be delivered
    /// to a webhook carrying these filters. AND across present
    /// dimensions; an absent dimension imposes no constraint.
    pub(crate) fn matches(&self, ctx: &EventMatchContext) -> bool {
        match self.affiliate_id {
            None => true,
            Some(a) => ctx.affiliate_id == Some(a),
        }
    }
}

/// The event-side values a webhook's [`WebhookFilters`] are matched
/// against. One field per supported filter dimension; defaults to "no
/// context" (all `None`), which only matches an unfiltered webhook.
/// Built per-event by the dispatcher — conversions populate
/// `affiliate_id` from the last-touch credited link; other event types
/// carry none today (so an affiliate-filtered webhook only ever matches
/// conversions).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EventMatchContext {
    pub affiliate_id: Option<AffiliateId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    #[serde(rename = "_id")]
    pub id: WebhookId,
    pub tenant_id: TenantId,
    pub url: String,
    pub secret: String,
    pub events: Vec<WebhookEventType>,
    /// Optional delivery narrowing on top of `events`. Defaults to empty
    /// (receive all) for rows created before filters existed.
    #[serde(default, skip_serializing_if = "WebhookFilters::is_empty")]
    pub filters: WebhookFilters,
    pub active: bool,
    pub created_at: DateTime,
}

/// Register a URL to receive real-time HMAC-signed POST requests when clicks or attributions occur.
/// The signing secret is returned once at creation time — save it to verify webhook signatures.
/// Payloads are JSON with `event`, `timestamp`, and `data` fields. Maximum 2 webhooks per tenant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    /// HTTPS URL to receive webhook POST requests.
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
    /// Optional delivery filters. Omit (or send `{}`) to receive every
    /// subscribed event. Currently supports `affiliate_id` to restrict
    /// conversions to one affiliate's links.
    #[serde(default)]
    pub filters: WebhookFilters,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWebhookResponse {
    pub id: WebhookId,
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
    /// HMAC-SHA256 signing secret. Use this to verify webhook payloads. Shown only once at creation time.
    #[schema(example = "whsec_k7J2mN9pQ4rT1vX8yB3cF6gH0")]
    pub secret: String,
    /// Delivery filters applied to this webhook. Omitted when empty.
    #[serde(skip_serializing_if = "WebhookFilters::is_empty")]
    pub filters: WebhookFilters,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookDetail {
    pub id: WebhookId,
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
    /// Delivery filters applied to this webhook. Omitted when empty.
    #[serde(skip_serializing_if = "WebhookFilters::is_empty")]
    pub filters: WebhookFilters,
    #[schema(example = true)]
    pub active: bool,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookDetail>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWebhookRequest {
    /// Enable / disable delivery. Omit to leave unchanged.
    #[serde(default)]
    #[schema(example = false)]
    pub active: Option<bool>,
    /// Replace the subscribed event types. Omit to leave unchanged; pass
    /// an empty array to reject (must subscribe to at least one event).
    #[serde(default)]
    pub events: Option<Vec<WebhookEventType>>,
    /// Replace the delivery URL. Must be HTTPS with a host. Omit to leave
    /// unchanged. The signing secret is NOT rotated — patching URL keeps
    /// the existing secret intact, which is the whole point versus
    /// delete + recreate.
    #[serde(default)]
    #[schema(example = "https://api.example.com/webhooks/rift")]
    pub url: Option<String>,
}

// ── Errors ──

use crate::services::auth::permissions::AuthzError;
use crate::services::billing::quota::QuotaError;

#[derive(Debug)]
pub enum WebhookError {
    QuotaExceeded(QuotaError),
    Forbidden(AuthzError),
    /// `filters.affiliate_id` referenced an affiliate that doesn't exist
    /// for this tenant (or affiliates aren't configured). Surfaced as a
    /// 400 so a typo'd filter fails fast instead of silently never firing.
    AffiliateNotFound,
    Internal(String),
}

impl From<QuotaError> for WebhookError {
    fn from(err: QuotaError) -> Self {
        WebhookError::QuotaExceeded(err)
    }
}

impl From<AuthzError> for WebhookError {
    fn from(err: AuthzError) -> Self {
        WebhookError::Forbidden(err)
    }
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::Forbidden(e) => write!(f, "{e}"),
            Self::AffiliateNotFound => write!(f, "Affiliate not found"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
