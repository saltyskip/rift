use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::core::public_id::{AffiliateId, TenantId};
use crate::core::threat_feed::ThreatFeed;
use crate::services::affiliates::repo::AffiliatesRepository;
use crate::services::app_users::repo::AppUsersRepository;
use crate::services::billing::quota::QuotaChecker;
use crate::services::billing::service::TierResolver;
use crate::services::domains::repo::DomainsRepository;
use crate::services::install_events::repo::InstallEventsRepository;
use crate::services::links::repo::LinksRepository;

/// Construction-time dependencies for [`super::service::LinksService`].
///
/// Bundled into a struct so callers use field-init shorthand at the call site
/// (clippy::too_many_arguments — see CLAUDE.md "if clippy complains about too
/// many arguments, use a struct or builder pattern").
pub struct LinksServiceDeps {
    pub links_repo: Arc<dyn LinksRepository>,
    pub domains_repo: Option<Arc<dyn DomainsRepository>>,
    pub affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
    /// `app_users` is the new user-scoped identity table. Optional during
    /// the Phase 1 cutover so reduced-feature builds without a database
    /// can still construct the service.
    pub app_users_repo: Option<Arc<dyn AppUsersRepository>>,
    /// Server-derived lifecycle stream (created / opened / identified /
    /// reinstalled / new_device). Optional for the same reason as
    /// `app_users_repo`.
    pub install_events_repo: Option<Arc<dyn InstallEventsRepository>>,
    pub threat_feed: ThreatFeed,
    pub public_url: String,
    pub quota: Option<Arc<dyn QuotaChecker>>,
    pub tiers: Option<Arc<dyn TierResolver>>,
}

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
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
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
    pub id: crate::core::public_id::LinkInternalId,
    /// Tenant who owns this link.
    pub tenant_id: crate::core::public_id::TenantId,
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
    pub affiliate_id: Option<crate::core::public_id::AffiliateId>,
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
    pub tenant_id: TenantId,
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

/// One row per `/lifecycle/attribute` call. Stored in a MongoDB
/// time-series collection (timeField = `timestamp`, metaField = `meta`).
/// Immutable — re-attribution to a new link is recorded as a NEW event
/// rather than mutating an existing one. The time-series shape gives
/// efficient time-bucketed analytics queries plus tier-based retention
/// via partial TTL indexes on `meta.retention_bucket`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEvent {
    pub timestamp: DateTime,
    pub meta: AttributionEventMeta,
    pub link_id: String,
    pub app_version: String,
}

/// Time-series metadata. `tenant_id` + `install_id` are the join keys
/// for queries; `retention_bucket` participates in per-tier TTL indexes;
/// `user_id` is mutated post-hoc by the identify backfill (Mongo
/// time-series only supports updates on meta-field paths).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEventMeta {
    pub tenant_id: TenantId,
    pub install_id: String,
    /// Retention tier marker — stamped at insert from the tenant's plan,
    /// used by `ensure_retention_ttl_indexes`. Stays with the event for
    /// life; tier upgrades don't extend historical retention.
    pub retention_bucket: String,
    /// User id once the install identifies. `None` for events written
    /// while the install was anonymous; backfilled to the bound user_id
    /// on `/lifecycle/identify` (Mongo can mutate meta-field paths but
    /// not arbitrary data fields on time-series — that's why this
    /// lives here and not at the top level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Result of `LinksService::identify_install`.
///
/// Drives the route handler's response + webhook decisions. `Created`
/// and `InstallAdded` are real state changes that fire the `identify`
/// webhook (and carry the user's credited links so the payload can
/// include them without a second query); `AlreadyPresent` is an
/// idempotent replay that returns 200 without firing. Conflict cases
/// (install already bound to a different user) surface via
/// `LinkError::IdentifyConflict`, not this enum.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentifyOutcome {
    /// First identify for this user_id — `app_users` row created with
    /// `install_id` in `install_ids`.
    Created(CreditedLinks),
    /// User existed; this install_id was a new device or reinstall.
    InstallAdded(CreditedLinks),
    /// User existed and already had this install_id bound. No-op.
    AlreadyPresent,
}

/// Attribution credit model applied at read time by the stats endpoint.
///
/// Defines which installs count toward a given link set:
///
/// - `LastTouch` (default): install's most-recent attribute event in the
///   query window has `link_id` in the set. Matches the marketer's mental
///   model of "which campaign closed this user."
/// - `FirstTouch`: install's first attribute event in the window has
///   `link_id` in the set. The acquisition-source model.
/// - `Touched`: install has any attribute event with `link_id` in the
///   set. Most generous — counts every install that ever encountered the
///   link, including those credited elsewhere by stricter models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditModel {
    FirstTouch,
    LastTouch,
    Touched,
}

impl CreditModel {
    /// Parse the `?credit=` query value. Unknown / missing → default
    /// (`LastTouch`).
    pub fn parse(s: Option<&str>) -> Self {
        match s.unwrap_or("last_touch") {
            "first_touch" => Self::FirstTouch,
            "touched" => Self::Touched,
            _ => Self::LastTouch,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FirstTouch => "first_touch",
            Self::LastTouch => "last_touch",
            Self::Touched => "touched",
        }
    }
}

/// Credited links for a user, resolved by walking their
/// `attribution_events` chain. The repo's
/// [`crate::services::links::repo::LinksRepository::credited_links_for_user`]
/// returns this with only the `*_link_id` fields populated; the
/// service layer enriches `*_link_metadata` via cached
/// `find_link_by_tenant_and_id` before handing it to the webhook
/// fire-sites. Both flavours are returned so receivers using either
/// credit model get the canonical metadata in one delivery — no
/// follow-up query to Rift required.
///
/// All four fields are `None` when the user has no attribution events
/// at or before the cutoff timestamp (e.g. a backend-fired conversion
/// for a user whose SDK has never called `/lifecycle/attribute`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CreditedLinks {
    pub first_touch_link_id: Option<String>,
    pub first_touch_link_metadata: Option<serde_json::Value>,
    pub last_touch_link_id: Option<String>,
    pub last_touch_link_metadata: Option<serde_json::Value>,
}

// ── Internal Types ──

/// Parameters for creating a new link (passed to repository).
pub struct CreateLinkInput {
    pub tenant_id: TenantId,
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<Document>,
    pub affiliate_id: Option<AffiliateId>,
    pub expires_at: Option<DateTime>,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
}

/// Fluent builder for `CreateLinkInput`. Setters accept `Option<T>` directly
/// so callers can propagate optionality from a request struct without
/// `if let` chains at the call site:
///
/// ```ignore
/// CreateLinkInput::new(tenant_id, link_id)
///     .web_url(req.web_url)
///     .ios_deep_link(req.ios_deep_link)
///     .metadata(metadata_doc)
/// ```
impl CreateLinkInput {
    pub fn new(tenant_id: TenantId, link_id: String) -> Self {
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

    pub fn ios_deep_link(mut self, v: Option<String>) -> Self {
        self.ios_deep_link = v;
        self
    }

    pub fn android_deep_link(mut self, v: Option<String>) -> Self {
        self.android_deep_link = v;
        self
    }

    pub fn web_url(mut self, v: Option<String>) -> Self {
        self.web_url = v;
        self
    }

    pub fn ios_store_url(mut self, v: Option<String>) -> Self {
        self.ios_store_url = v;
        self
    }

    pub fn android_store_url(mut self, v: Option<String>) -> Self {
        self.android_store_url = v;
        self
    }

    pub fn metadata(mut self, v: Option<Document>) -> Self {
        self.metadata = v;
        self
    }

    pub fn affiliate_id(mut self, v: Option<AffiliateId>) -> Self {
        self.affiliate_id = v;
        self
    }

    pub fn expires_at(mut self, v: Option<DateTime>) -> Self {
        self.expires_at = v;
        self
    }

    pub fn agent_context(mut self, v: Option<AgentContext>) -> Self {
        self.agent_context = v;
        self
    }

    pub fn social_preview(mut self, v: Option<SocialPreview>) -> Self {
        self.social_preview = v;
        self
    }
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
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
    pub affiliate_id: Option<crate::core::public_id::AffiliateId>,
    /// Structured context for AI agents. When set, agents resolving this link receive action, CTA, and description metadata alongside the destinations.
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
    /// Public Open Graph/Twitter preview data rendered on Rift landing pages.
    #[serde(default)]
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct CreateLinkResponse {
    /// The link's vanity slug or auto-generated ID — the path segment after the domain.
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    /// The fully-qualified link URL ready to share.
    #[schema(example = "https://riftl.ink/summer-menu-2025")]
    pub url: String,
    /// When this link expires (RFC 3339). Null for permanent links. Links without a verified custom domain expire after 30 days.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2025-07-15T10:30:00Z")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct LinkDetail {
    /// The link's vanity slug or auto-generated ID — the path segment after the domain.
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    /// The fully-qualified link URL ready to share.
    #[schema(example = "https://riftl.ink/summer-menu-2025")]
    pub url: String,
    /// iOS deep link URI this link redirects to on iOS devices with the app installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI this link redirects to on Android devices with the app installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop / unknown platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    /// App Store URL for iOS users without the app installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    /// Play Store URL for Android users without the app installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    /// When this link was created (RFC 3339).
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
    /// Affiliate this link is attributed to. None for unattributed links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliate_id: Option<AffiliateId>,
    /// Structured context for AI agents resolving this link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<AgentContext>,
    /// Open Graph / Twitter preview metadata rendered on Rift landing pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub social_preview: Option<SocialPreview>,
}

// ── Bulk Create ──

/// Template applied to every link in a bulk-create request. Same fields as
/// `CreateLinkRequest` minus `custom_id` (the bulk endpoint generates or
/// accepts a list of IDs separately) and minus `affiliate_id` (the caller's
/// scope governs attribution for the whole batch).
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct BulkLinkTemplate {
    /// iOS deep link URI applied to every link in the batch (e.g. "myapp://product/123").
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI applied to every link in the batch.
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    /// Web fallback URL applied to every link (desktop / unknown platforms).
    #[serde(default)]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    /// App Store link for iOS applied to every link in the batch.
    #[serde(default)]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android applied to every link in the batch.
    #[serde(default)]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata copied onto every link in the batch.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Affiliate this whole batch should be attributed to. Optional for full-scope
    /// callers; ignored / overridden for affiliate-scoped callers.
    #[serde(default)]
    pub affiliate_id: Option<crate::core::public_id::AffiliateId>,
    /// Structured context for AI agents applied to every link in the batch.
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
    /// Open Graph / Twitter preview data applied to every link in the batch.
    #[serde(default)]
    pub social_preview: Option<SocialPreview>,
}

/// Bulk create request. Exactly one of `custom_ids` or `count` must be set.
/// `custom_ids` mode: caller-supplied vanity slugs (each 3-64 chars,
/// alphanumeric + hyphens). `count` mode: server generates N random 8-char
/// uppercase IDs.
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct BulkCreateLinksRequest {
    /// Shared destinations and metadata applied to every link in the batch.
    pub template: BulkLinkTemplate,
    /// Caller-supplied vanity slugs (3-64 chars, alphanumeric + hyphens).
    /// Mutually exclusive with `count`.
    #[serde(default)]
    pub custom_ids: Option<Vec<String>>,
    /// Number of links to generate with auto-assigned 8-char IDs.
    /// Mutually exclusive with `custom_ids`. Max 100 per batch.
    #[serde(default)]
    #[schema(example = 50)]
    pub count: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct BulkLinkResult {
    /// The link's vanity slug or auto-generated ID — the path segment after the domain.
    #[schema(example = "partner-acme")]
    pub link_id: String,
    /// The fully-qualified link URL ready to share.
    #[schema(example = "https://go.acme.com/partner-acme")]
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct BulkCreateLinksResponse {
    /// Every link successfully created by the batch, in input order.
    pub links: Vec<BulkLinkResult>,
}

/// Per-row failure surfaced when a batch is rejected without inserting
/// anything. The full list is returned in one response so the caller can fix
/// every problem in one pass and retry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchItemError {
    #[schema(example = 4)]
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "promo-2")]
    pub custom_id: Option<String>,
    #[schema(example = "link_id_taken")]
    pub code: String,
    #[schema(example = "'promo-2' is already taken")]
    pub message: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListLinksQuery {
    /// Maximum number of links to return (1-100, default 50).
    pub limit: Option<i64>,
    /// Cursor for pagination — pass `next_cursor` from the previous response.
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct ListLinksResponse {
    /// The current page of links, most recent first.
    pub links: Vec<LinkDetail>,
    /// Cursor for the next page. Null if no more results.
    #[schema(example = "lnk_665a1b2c3d4e5f6a7b8c9d0e")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct IdentifyRequest {
    #[schema(example = "d4f7a1b2-3c8e-4f9a-b5d6-7e8f9a0b1c2d")]
    pub install_id: String,
    #[schema(example = "user_12345")]
    pub user_id: String,
}

/// Response from `POST /v1/lifecycle/attribute`. Today: `{success}` only.
/// Forward room: `event_id` (stable id customers can dedup against),
/// `is_first_touch` (signal whether `installs` row was newly inserted).
#[derive(Debug, Serialize, ToSchema)]
pub struct AttributeResponse {
    #[schema(example = true)]
    pub success: bool,
}

/// Response from `PUT /v1/lifecycle/identify`. Today: `{success}` only.
/// Forward room: `prior_attributions` (anonymous events just linked to
/// this user_id), `bound_at` (server-side timestamp of the binding).
#[derive(Debug, Serialize, ToSchema)]
pub struct IdentifyResponse {
    #[schema(example = true)]
    pub success: bool,
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
pub struct AttributeRequest {
    #[schema(example = "summer-menu-2025")]
    pub link_id: String,
    #[schema(example = "d4f7a1b2-3c8e-4f9a-b5d6-7e8f9a0b1c2d")]
    pub install_id: String,
    #[schema(example = "2.4.1")]
    pub app_version: String,
    /// Optional device + app context captured by the SDK from public OS
    /// APIs (no permissions required). Used server-side to enrich
    /// `install.created` events and to distinguish `install.reinstalled`
    /// from `install.new_device` on identify. Absent on older SDK
    /// versions that pre-date the context-capture release.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<AttributeContext>,
}

/// Device / app context captured by the SDK. Mirror of
/// [`crate::services::install_events::models::InstallContext`] at the
/// API boundary — kept as a separate type so OpenAPI schemas don't leak
/// internal storage types.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AttributeContext {
    #[schema(example = "2.4.1")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub app_version: Option<String>,
    #[schema(example = "ios")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub platform: Option<String>,
    #[schema(example = "iOS")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub os: Option<String>,
    #[schema(example = "17.4.1")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub os_version: Option<String>,
    #[schema(example = "iPhone15,4")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub device_model: Option<String>,
    #[schema(example = "Apple")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub device_manufacturer: Option<String>,
    #[schema(example = "en_US")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    #[schema(example = "US")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<String>,
    #[schema(example = "America/Los_Angeles")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
}

impl From<AttributeContext> for crate::services::install_events::models::InstallContext {
    fn from(c: AttributeContext) -> Self {
        Self {
            app_version: c.app_version,
            platform: c.platform,
            os: c.os,
            os_version: c.os_version,
            device_model: c.device_model,
            device_manufacturer: c.device_manufacturer,
            locale: c.locale,
            region: c.region,
            timezone: c.timezone,
        }
    }
}

// ── Errors ──

use crate::services::auth::permissions::AuthzError;
use crate::services::billing::quota::QuotaError;
use std::fmt;

#[derive(Debug)]
pub enum LinkError {
    InvalidCustomId(String),
    InvalidUrl(String),
    InvalidMetadata(String),
    InvalidAgentContext(String),
    InvalidSocialPreview(String),
    ThreatDetected(String),
    LinkIdTaken(String),
    NotFound,
    NoVerifiedDomain,
    EmptyUpdate,
    /// Caller's affiliate scope does not match the requested `affiliate_id`.
    AffiliateScopeMismatch,
    /// Caller (full scope) referenced an affiliate that doesn't exist in this tenant.
    AffiliateNotFound,
    Forbidden(AuthzError),
    QuotaExceeded(QuotaError),
    /// `/lifecycle/identify` rejected because the install_id is already
    /// bound to a different user. Carries the existing user_id for
    /// observability (the route does NOT echo it to the client). Returns
    /// HTTP 409.
    IdentifyConflict {
        existing_user_id: String,
    },
    Internal(String),
    // ── Bulk-create only ──
    BatchTooLarge {
        max: usize,
        got: usize,
    },
    BatchEmpty,
    /// Both `custom_ids` and `count` were set; only one is allowed.
    BatchModeAmbiguous,
    /// Neither `custom_ids` nor `count` was set; one is required.
    BatchModeMissing,
    /// One or more rows failed validation. Full list returned to the caller
    /// so every problem can be fixed in one pass.
    BatchValidationFailed(Vec<BatchItemError>),
}

impl From<QuotaError> for LinkError {
    fn from(err: QuotaError) -> Self {
        LinkError::QuotaExceeded(err)
    }
}

impl From<AuthzError> for LinkError {
    fn from(err: AuthzError) -> Self {
        LinkError::Forbidden(err)
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCustomId(e) => write!(f, "{e}"),
            Self::InvalidUrl(e) => write!(f, "{e}"),
            Self::InvalidMetadata(e) => write!(f, "{e}"),
            Self::InvalidAgentContext(e) => write!(f, "{e}"),
            Self::InvalidSocialPreview(e) => write!(f, "{e}"),
            Self::ThreatDetected(e) => write!(f, "{e}"),
            Self::LinkIdTaken(id) => write!(f, "'{id}' is already taken"),
            Self::NotFound => write!(f, "Link not found"),
            Self::NoVerifiedDomain => {
                write!(f, "Custom IDs require a verified custom domain")
            }
            Self::EmptyUpdate => write!(f, "No fields to update"),
            Self::AffiliateScopeMismatch => write!(
                f,
                "affiliate_id does not match the affiliate this credential is scoped to"
            ),
            Self::AffiliateNotFound => write!(f, "Affiliate not found"),
            Self::Forbidden(e) => write!(f, "{e}"),
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::IdentifyConflict { .. } => {
                write!(f, "install_id is already bound to a different user")
            }
            Self::Internal(e) => write!(f, "Internal error: {e}"),
            Self::BatchTooLarge { max, got } => {
                write!(f, "Batch too large: {got} items (max {max})")
            }
            Self::BatchEmpty => write!(f, "Batch is empty"),
            Self::BatchModeAmbiguous => {
                write!(f, "Specify exactly one of `custom_ids` or `count`")
            }
            Self::BatchModeMissing => {
                write!(f, "One of `custom_ids` or `count` is required")
            }
            Self::BatchValidationFailed(errs) => {
                write!(f, "{} item(s) failed validation", errs.len())
            }
        }
    }
}

impl LinkError {
    /// Machine-readable error code for API responses.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidCustomId(_) => "invalid_custom_id",
            Self::InvalidUrl(_) => "invalid_url",
            Self::InvalidMetadata(_) => "invalid_metadata",
            Self::InvalidAgentContext(_) => "invalid_agent_context",
            Self::InvalidSocialPreview(_) => "invalid_social_preview",
            Self::ThreatDetected(_) => "threat_detected",
            Self::LinkIdTaken(_) => "link_id_taken",
            Self::NotFound => "not_found",
            Self::NoVerifiedDomain => "no_verified_domain",
            Self::EmptyUpdate => "empty_update",
            Self::AffiliateScopeMismatch => "affiliate_scope_mismatch",
            Self::AffiliateNotFound => "affiliate_not_found",
            Self::Forbidden(e) => e.code(),
            Self::QuotaExceeded(_) => "quota_exceeded",
            Self::IdentifyConflict { .. } => "identify_conflict",
            Self::Internal(_) => "db_error",
            Self::BatchTooLarge { .. } => "batch_too_large",
            Self::BatchEmpty => "batch_empty",
            Self::BatchModeAmbiguous => "batch_mode_ambiguous",
            Self::BatchModeMissing => "batch_mode_missing",
            Self::BatchValidationFailed(_) => "invalid_batch",
        }
    }
}

/// Outcome of an atomic bulk insert. `DuplicateLinkIds` carries the input
/// indices that collided with the unique `(tenant_id, link_id)` index — the
/// service maps these back to per-row `link_id_taken` errors so the caller
/// learns every conflict in one round trip. The transaction is rolled back
/// before this is returned, so no partial inserts persist.
#[derive(Debug)]
pub enum BulkInsertError {
    DuplicateLinkIds(Vec<usize>),
    Internal(String),
}
