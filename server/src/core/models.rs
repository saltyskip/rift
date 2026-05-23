//! Shared data shapes used by `core/` infrastructure. Currently holds the
//! outbound webhook event payloads — these cross multiple service slices
//! (clicks from links, attribution from links, conversions from
//! conversions) and don't belong inside any one domain's `models.rs`.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ClickEventPayload {
    pub tenant_id: String,
    pub link_id: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub platform: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttributeEventPayload {
    pub tenant_id: String,
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
    /// User id at fire time. `Some` when the install was already
    /// identified (re-attribution by an existing user — the
    /// existing-install campaign path). `None` for fresh installs that
    /// haven't completed `identify` yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Snapshot of `Link.metadata` at fire time. Free-form JSON; receivers
    /// agree on schema with the campaign creator (e.g. `{bonus_type,
    /// bonus_amount_usdc}`). Absent when the attributed link has no
    /// metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversionEventPayload {
    /// Stable unique ID for this event. Customer handlers should use it as an
    /// idempotency key to avoid double-counting on webhook delivery retries.
    pub event_id: String,
    pub tenant_id: String,
    pub source_id: String,
    pub conversion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// First-touch link credit, computed from the user's
    /// `attribution_events` chain at fire time. Absent if the user has
    /// no attribution events at or before the conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_touch_link_id: Option<String>,
    /// Snapshot of `Link.metadata` for `first_touch_link_id` at fire
    /// time. Receivers using first-touch credit (welcome bonuses,
    /// acquisition-source crediting) read this directly. Absent when
    /// `first_touch_link_id` is absent or the link has no metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_touch_link_metadata: Option<serde_json::Value>,
    /// Last-touch link credit (the conversion-closer in marketer
    /// terms). Same provenance and absence rules as `first_touch_link_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_touch_link_id: Option<String>,
    /// Snapshot of `Link.metadata` for `last_touch_link_id` at fire
    /// time. Receivers using last-touch credit read this directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_touch_link_metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentifyEventPayload {
    pub tenant_id: String,
    pub user_id: String,
    pub install_id: String,
    /// First-touch link credit at the moment of identify, computed from
    /// the user's now-unified `attribution_events` chain (the
    /// pre-identify backfill has already stamped `user_id` on prior
    /// anonymous rows). Absent when the user has no prior attribution
    /// events — typical for "signed up before tapping a link."
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_touch_link_id: Option<String>,
    /// Snapshot of `Link.metadata` for `first_touch_link_id`. The
    /// canonical payload for first-touch-credit receivers (welcome
    /// bonus campaigns, etc.) — strictly more useful than firing the
    /// id alone since most receivers immediately query the link
    /// metadata to decide what to do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_touch_link_metadata: Option<serde_json::Value>,
    /// Last-touch link credit at identify time. Same provenance and
    /// absence rules as `first_touch_link_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_touch_link_id: Option<String>,
    /// Snapshot of `Link.metadata` for `last_touch_link_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_touch_link_metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}
