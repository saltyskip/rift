use mongodb::bson::{oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

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

/// Install state — one row per `(tenant_id, install_id)`. Holds the
/// first-touch link attribution and (optionally) the bound user_id. This
/// is the **mutable** projection over the immutable `attribution_events`
/// time-series log; the event log is the source of truth, this is the
/// materialized state for fast lookups (stats, conversions).
///
/// Updated by:
/// - `/v1/lifecycle/attribute`: upsert with `first_link_id` set on insert
///   only (preserves first-touch semantics for stats).
/// - `/v1/lifecycle/identify`: `$set user_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    /// Unique per install (generated client-side).
    pub install_id: String,
    /// First link that attributed this install. Set on insert only; later
    /// `/lifecycle/attribute` calls for the same install append to the
    /// event log but do not overwrite this field.
    pub first_link_id: String,
    /// App version recorded at first-touch.
    pub first_app_version: String,
    /// Soonest attribution timestamp for this install.
    pub first_attributed_at: DateTime,
    /// User id bound via `/lifecycle/identify`. None until the user
    /// authenticates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// When `user_id` was first bound (None until identify completes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identified_at: Option<DateTime>,
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
    /// User id at the moment of the event. Often `None` for the first
    /// event of an install (user hasn't signed in yet). Filled by the
    /// route handler if `installs.user_id` is already set, so downstream
    /// subscribers can act immediately on existing-install re-attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Time-series metadata. `tenant_id` + `install_id` are the join keys
/// for queries; `retention_bucket` participates in per-tier TTL indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEventMeta {
    pub tenant_id: ObjectId,
    pub install_id: String,
    /// Retention tier marker — stamped at insert from the tenant's plan,
    /// used by `ensure_retention_ttl_indexes`. Stays with the event for
    /// life; tier upgrades don't extend historical retention.
    pub retention_bucket: String,
}

/// Result of `LinksRepository::record_attribute_event`.
///
/// `FirstTouch` and `Retouch` both successfully record an event; they
/// differ only in whether `installs` was newly inserted (first-touch
/// attribution for this install) vs already existed (re-attribution to
/// a new or same link). Both fire the `attribute` webhook. The caller
/// uses the returned `Install` to build the outbound payload (need
/// `user_id` if it's already bound — that's the existing-install path).
#[derive(Debug, Clone)]
pub enum AttributeOutcome {
    FirstTouch(Install),
    Retouch(Install),
}

/// Result of `LinksRepository::identify_install`.
///
/// Distinguishes a real state change (`NewBind` — fire `identify` webhook)
/// from an idempotent replay (`AlreadyBound` — suppress). The carried
/// `Install` reflects the post-update state for payload construction.
#[derive(Debug, Clone)]
pub enum IdentifyOutcome {
    /// Bind changed state — `user_id` was previously absent and we just
    /// set it. Fire the `identify` webhook.
    NewBind(Install),
    /// Install was already bound to this same `user_id`. Route returns 200
    /// but webhook is suppressed.
    #[allow(dead_code)]
    AlreadyBound(Install),
    /// No install row matched. Either `install_id` is unknown for this
    /// tenant, or it's already bound to a different user — both surface
    /// as 404.
    NotFound,
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

    pub fn affiliate_id(mut self, v: Option<ObjectId>) -> Self {
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
    #[schema(value_type = String, example = "665a1b2c3d4e5f6a7b8c9d0e")]
    #[cfg_attr(feature = "mcp", schemars(with = "Option<String>"))]
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

// ── Bulk Create ──

/// Template applied to every link in a bulk-create request. Same fields as
/// `CreateLinkRequest` minus `custom_id` (the bulk endpoint generates or
/// accepts a list of IDs separately) and minus `affiliate_id` (the caller's
/// scope governs attribution for the whole batch).
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct BulkLinkTemplate {
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub ios_deep_link: Option<String>,
    #[serde(default)]
    #[schema(example = "tablefour://restaurant/782/reserve")]
    pub android_deep_link: Option<String>,
    #[serde(default)]
    #[schema(example = "https://tablefour.com/restaurant/782")]
    pub web_url: Option<String>,
    #[serde(default)]
    #[schema(example = "https://apps.apple.com/app/tablefour/id1234567890")]
    pub ios_store_url: Option<String>,
    #[serde(default)]
    #[schema(example = "https://play.google.com/store/apps/details?id=com.tablefour.app")]
    pub android_store_url: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Affiliate this whole batch should be attributed to. Optional for full-scope
    /// callers; ignored / overridden for affiliate-scoped callers.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "665a1b2c3d4e5f6a7b8c9d0e")]
    #[cfg_attr(feature = "mcp", schemars(with = "Option<String>"))]
    pub affiliate_id: Option<ObjectId>,
    #[serde(default)]
    pub agent_context: Option<AgentContext>,
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
    pub template: BulkLinkTemplate,
    #[serde(default)]
    pub custom_ids: Option<Vec<String>>,
    #[serde(default)]
    #[schema(example = 50)]
    pub count: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkLinkResult {
    #[schema(example = "partner-acme")]
    pub link_id: String,
    #[schema(example = "https://go.acme.com/partner-acme")]
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkCreateLinksResponse {
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
pub struct ListLinksResponse {
    pub links: Vec<LinkDetail>,
    /// Cursor for the next page. Null if no more results.
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
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
