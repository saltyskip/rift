use mongodb::bson::{oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// ── Database Documents ──

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    #[default]
    Active,
    Flagged,
    Disabled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgentContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    /// Tenant who owns this link (API key ObjectId).
    pub tenant_id: ObjectId,
    /// Short alphanumeric ID used in URLs (e.g. "ABCD1234").
    pub link_id: String,
    /// iOS deep link URI (e.g. "myapp://product/123").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    /// Arbitrary metadata (campaign name, source, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Document>,
    pub created_at: DateTime,
    /// Link safety status.
    #[serde(default)]
    pub status: LinkStatus,
    /// Reason the link was flagged/disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_reason: Option<String>,
    /// When this link expires (None = never).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent_context: Option<AgentContext>,
}

/// Click event stored in the `click_events` time series collection.
/// The `meta` subdocument is the metaField for time series bucketing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickMeta {
    pub tenant_id: ObjectId,
    pub link_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEvent {
    pub meta: ClickMeta,
    pub clicked_at: DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub link_id: String,
    /// Unique per install (generated client-side).
    pub install_id: String,
    /// User ID linked after signup (None until user authenticates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub app_version: String,
    pub attributed_at: DateTime,
}

// ── Internal Types ──

/// Parameters for creating a new link (passed to repository).
pub struct CreateLinkInput {
    pub tenant_id: ObjectId,
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<Document>,
    pub expires_at: Option<DateTime>,
    pub agent_context: Option<AgentContext>,
}

impl CreateLinkInput {
    pub fn new(tenant_id: ObjectId, link_id: String) -> Self {
        Self {
            tenant_id,
            link_id,
            ios_deep_link: None,
            android_deep_link: None,
            web_url: None,
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            expires_at: None,
            agent_context: None,
        }
    }

    pub fn expires_at(mut self, v: DateTime) -> Self {
        self.expires_at = Some(v);
        self
    }

    pub fn ios_deep_link(mut self, v: impl Into<String>) -> Self {
        self.ios_deep_link = Some(v.into());
        self
    }

    pub fn android_deep_link(mut self, v: impl Into<String>) -> Self {
        self.android_deep_link = Some(v.into());
        self
    }

    pub fn web_url(mut self, v: impl Into<String>) -> Self {
        self.web_url = Some(v.into());
        self
    }

    pub fn ios_store_url(mut self, v: impl Into<String>) -> Self {
        self.ios_store_url = Some(v.into());
        self
    }

    pub fn android_store_url(mut self, v: impl Into<String>) -> Self {
        self.android_store_url = Some(v.into());
        self
    }

    pub fn metadata(mut self, v: Document) -> Self {
        self.metadata = Some(v);
        self
    }

    pub fn agent_context(mut self, v: AgentContext) -> Self {
        self.agent_context = Some(v);
        self
    }
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLinkRequest {
    /// Optional vanity slug (3-64 chars, alphanumeric + hyphens).
    #[serde(default)]
    pub custom_id: Option<String>,
    /// iOS deep link URI (e.g. "myapp://product/123").
    #[serde(default)]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(default)]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    #[serde(default)]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(default)]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(default)]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateLinkResponse {
    pub link_id: String,
    pub url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLinkRequest {
    /// iOS deep link URI.
    #[serde(default)]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(default)]
    pub android_deep_link: Option<String>,
    /// Web fallback URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(default)]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(default)]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkDetail {
    pub link_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListLinksQuery {
    /// Maximum number of links to return (1-100, default 50).
    pub limit: Option<i64>,
    /// Cursor for pagination — pass `next_cursor` from the previous response.
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListLinksResponse {
    pub links: Vec<LinkDetail>,
    /// Cursor for the next page. Null if no more results.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportAttributionRequest {
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
    /// Optional custom domain for tenant-scoped link lookup.
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AttributionResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkAttributionRequest {
    pub install_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkStatsResponse {
    pub link_id: String,
    pub click_count: u64,
    pub install_count: u64,
    pub conversion_rate: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedLink {
    pub link_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
}

// ── SDK Click Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct SdkClickRequest {
    /// Link ID to resolve.
    pub link_id: String,
    /// Optional custom domain for tenant-scoped lookup.
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SdkClickResponse {
    pub link_id: String,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
}

// ── Deferred Deep Linking Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeferredLinkRequest {
    /// Link ID from clipboard or install referrer.
    pub link_id: String,
    /// Client-generated install ID for attribution.
    pub install_id: String,
    /// Optional custom domain for tenant-scoped link lookup.
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeferredLinkResponse {
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
}

// ── Timeseries Analytics Models ──

#[derive(Debug, Deserialize, IntoParams)]
pub struct TimeseriesQuery {
    /// Start of date range (RFC 3339). Defaults to 30 days ago.
    pub from: Option<String>,
    /// End of date range (RFC 3339). Defaults to now.
    pub to: Option<String>,
    /// Bucket granularity. Only "daily" supported.
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeseriesDataPoint {
    pub date: String,
    pub clicks: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TimeseriesResponse {
    pub link_id: String,
    pub granularity: String,
    pub from: String,
    pub to: String,
    pub data: Vec<TimeseriesDataPoint>,
}
