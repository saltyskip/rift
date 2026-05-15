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
    pub link_id: String,
    pub conversion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentifyEventPayload {
    pub tenant_id: String,
    pub user_id: String,
    pub link_id: String,
    pub install_id: String,
    /// Snapshot of `Link.metadata` at fire time. Free-form JSON; receivers
    /// agree on schema with the campaign creator (e.g. `{bonus_type,
    /// bonus_amount_usdc}`). Absent when the attributed link has no metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}
