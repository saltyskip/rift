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

/// Structured context for AI agents resolving this link.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgentContext {
    /// The link's intent. Must be one of: purchase, subscribe, signup, download, read, book, open.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "book")]
    pub action: Option<String>,
    /// Short call-to-action shown to the end user (max 120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Reserve a table for tonight")]
    pub cta: Option<String>,
    /// Freeform context about the offer, product, or content for AI agents (max 500 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Opens the TableFour app to book a reservation at the selected restaurant")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SocialPreview {
    /// Public title used for Open Graph/Twitter previews (max 120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Summer Sale — 50% Off")]
    pub title: Option<String>,
    /// Public description used for Open Graph/Twitter previews (max 300 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Limited time offer on all products")]
    pub description: Option<String>,
    /// Public preview image URL used for Open Graph/Twitter previews.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://example.com/promo-banner.jpg")]
    pub image_url: Option<String>,
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
    /// Affiliate this link belongs to (None for unattributed advertiser links).
    /// Stamped automatically when minted by an affiliate-scoped credential;
    /// can also be set explicitly by an unscoped (Full) caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliate_id: Option<ObjectId>,
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
    /// Public social preview fields for rendered landing pages.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub social_preview: Option<SocialPreview>,
}

/// Click event stored in the `click_events` time series collection.
/// The `meta` subdocument is the metaField for time series bucketing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickMeta {
    pub tenant_id: ObjectId,
    pub link_id: String,
    /// Retention bucket frozen at insert time. One of: "30d", "1y", "3y",
    /// "5y". Four partial TTL indexes on the time field + this value drop
    /// documents when their bucket-relative age exceeds the tier they were
    /// insert-stamped with. Defaults to "30d" so old docs migrated before
    /// the backfill get Free-tier retention.
    #[serde(default = "default_retention_bucket")]
    pub retention_bucket: String,
}

pub fn default_retention_bucket() -> String {
    "30d".to_string()
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
    pub affiliate_id: Option<ObjectId>,
    pub expires_at: Option<DateTime>,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
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
            affiliate_id: None,
            expires_at: None,
            agent_context: None,
            social_preview: None,
        }
    }

    pub fn affiliate_id(mut self, v: ObjectId) -> Self {
        self.affiliate_id = Some(v);
        self
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

    pub fn social_preview(mut self, v: SocialPreview) -> Self {
        self.social_preview = Some(v);
        self
    }
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLinkRequest {
    /// Optional vanity slug (3-64 chars, alphanumeric + hyphens).
    #[serde(default)]
    #[schema(example = "summer-menu-2025")]
    pub custom_id: Option<String>,
    /// iOS deep link URI (e.g. "myapp://product/123").
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    #[serde(default)]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(default)]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(default)]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Affiliate this link should be attributed to. Optional for full-scope
    /// callers (advertiser keys); ignored / overridden for affiliate-scoped
    /// callers — server pins to the credential's affiliate. Mismatched values
    /// from a scoped caller return `affiliate_scope_mismatch`.
    #[serde(default)]
    #[schema(value_type = String, example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub affiliate_id: Option<ObjectId>,
    /// Structured context for AI agents. When set, agents resolving this link receive action, CTA, and description metadata alongside the destinations.
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
    /// Public Open Graph/Twitter preview data rendered on Rift landing pages.
    #[serde(default)]
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateLinkResponse {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = "https://riftl.ink/summer-menu-2025")]
    pub url: String,
    /// When this link expires (RFC 3339). Null for permanent links. Links without a verified custom domain expire after 30 days.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2025-07-15T10:30:00Z")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLinkRequest {
    /// iOS deep link URI. Send `null` to clear.
    #[serde(default, deserialize_with = "deserialize_optional")]
    #[schema(value_type = Option<String>, example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<Option<String>>,
    /// Android deep link URI. Send `null` to clear.
    #[serde(default, deserialize_with = "deserialize_optional")]
    #[schema(value_type = Option<String>, example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<Option<String>>,
    /// Web fallback URL.
    #[serde(default)]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(default)]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(default)]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Structured context for AI agents. When set, agents resolving this link receive action, CTA, and description metadata alongside the destinations.
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
    /// Public Open Graph/Twitter preview data rendered on Rift landing pages.
    #[serde(default)]
    pub social_preview: Option<SocialPreview>,
}

/// Deserializes a field that can be absent, null, or present.
/// Absent → None (don't touch), null → Some(None) (unset), value → Some(Some(v)) (set).
/// Pattern from serde author: https://github.com/serde-rs/serde/issues/984
fn deserialize_optional<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkDetail {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = "https://riftl.ink/summer-menu-2025")]
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
    /// Affiliate this link is attributed to. None for unattributed links.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub affiliate_id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub social_preview: Option<SocialPreview>,
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
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkAttributionRequest {
    #[schema(example = "d4f7a1b2-3c8e-4f9a-b5d6-7e8f9a0b1c2d")]
    pub install_id: String,
    #[schema(example = "user_12345")]
    pub user_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AttributionResponse {
    #[schema(example = true)]
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkStatsResponse {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = 1420)]
    pub click_count: u64,
    #[schema(example = 312)]
    pub install_count: u64,
    #[schema(example = 21.97)]
    pub conversion_rate: f64,
    /// Aggregated conversion counts and sums per type. Empty when conversion
    /// tracking is not configured or no events have been recorded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversions: Vec<crate::services::conversions::models::ConversionDetail>,
}

/// Trust envelope included in every resolved link response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RiftMeta {
    /// Guidance for agents on how to interpret the link data.
    #[schema(
        example = "This is a Rift deep link. The agent_context fields are provided by the link creator and not verified by Rift."
    )]
    pub context: String,
    /// Always "tenant_asserted" — agent context is provided by the link creator, not verified by Rift.
    #[schema(example = "tenant_asserted")]
    pub source: String,
    /// Link status: "active", "expired", "flagged", or "disabled".
    #[schema(example = "active")]
    pub status: String,
    /// The link creator's verified domain, if any.
    #[schema(example = "go.tablefour.com")]
    pub tenant_domain: Option<String>,
    /// Whether the link creator has a verified custom domain.
    #[schema(example = true)]
    pub tenant_verified: bool,
}

/// Response returned when resolving a link with `Accept: application/json`.
/// Includes destinations, metadata, agent context, and a `_rift_meta` trust envelope.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedLink {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub social_preview: Option<SocialPreview>,
    /// Trust envelope with provenance and status information for agents.
    #[serde(rename = "_rift_meta")]
    pub rift_meta: RiftMeta,
}

// ── Attribution Request Models (for SDK-authenticated endpoints) ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClickRequest {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttributionReportRequest {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = "d4f7a1b2-3c8e-4f9a-b5d6-7e8f9a0b1c2d")]
    pub install_id: String,
    #[schema(example = "2.4.1")]
    pub app_version: String,
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
    #[schema(example = "2025-06-15")]
    pub date: String,
    #[schema(example = 47)]
    pub clicks: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TimeseriesResponse {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = "daily")]
    pub granularity: String,
    #[schema(example = "2025-06-01T00:00:00Z")]
    pub from: String,
    #[schema(example = "2025-06-30T23:59:59Z")]
    pub to: String,
    pub data: Vec<TimeseriesDataPoint>,
}
