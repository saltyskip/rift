use mongodb::bson::{oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Source types ──

/// The kind of source, which determines how incoming webhook payloads are parsed.
/// v1 ships `custom` only. Future integrations (RevenueCat, Stripe, etc.) are added
/// by implementing a new `ConversionParser`, adding a variant here, and one line in
/// `parser_for`. No schema migration required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Custom,
}

// ── Database documents ──

/// A webhook receiver for incoming conversion events. Each source has an opaque
/// URL token that forms its webhook URL. Future integration source types will
/// populate `signing_secret` and `config`; custom sources leave them empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub name: String,
    pub source_type: SourceType,
    /// 32-byte random hex — forms the public webhook URL path `POST /w/{url_token}`.
    pub url_token: String,
    /// HMAC secret for verifying signatures on integration sources (RevenueCat, Stripe, etc.).
    /// None for custom sources — the opaque URL token is the auth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_secret: Option<String>,
    /// Type-specific config. Empty `{}` for custom in v1.
    pub config: Document,
    pub created_at: DateTime,
}

/// A single conversion event. Stored in the `conversion_events` MongoDB time series
/// collection — the source of truth. Stats/counters are computed on read via
/// aggregation pipelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionEvent {
    pub meta: ConversionMeta,
    /// Time the event occurred. For integration parsers this may be extracted from
    /// the upstream event (e.g. Stripe's `created`); for custom sources it defaults to now.
    pub occurred_at: DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Up to 1KB of caller-defined data. Stored verbatim, exposed via the outbound
    /// webhook, never indexed or queried in v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Document>,
}

/// Meta fields for the time series bucket. Fields placed here are efficient to
/// `$match` against (MongoDB buckets documents by meta values). Non-meta fields
/// are stored but less efficient to filter on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionMeta {
    pub tenant_id: ObjectId,
    pub link_id: String,
    pub source_id: ObjectId,
    pub conversion_type: String,
    /// Retention bucket frozen at insert time — see ClickMeta for details.
    #[serde(default = "crate::services::links::models::default_retention_bucket")]
    pub retention_bucket: String,
}

/// Idempotency dedup record. One row per `(tenant_id, idempotency_key)` that
/// Rift has ever seen (within the TTL window). See `conversion_dedup` discussion
/// in the plan for why this is a separate collection from `conversion_events`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionDedup {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub idempotency_key: String,
    pub created_at: DateTime,
}

// ── API request / response DTOs ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceRequest {
    /// Human-readable name. Unique per tenant.
    #[schema(example = "backend-deposits")]
    pub name: String,
    /// Source type. v1 supports `custom` only.
    #[schema(example = "custom")]
    pub source_type: SourceType,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateSourceResponse {
    #[schema(example = "66a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "backend-deposits")]
    pub name: String,
    pub source_type: SourceType,
    /// The public webhook URL for this source. Include this in your backend or
    /// integration webhook config. The URL token is unguessable; the URL itself
    /// is the auth. Rotate by deleting + recreating the source.
    #[schema(example = "https://api.riftl.ink/w/a1b2c3d4e5f6...")]
    pub webhook_url: String,
    #[schema(example = "2026-04-10T12:00:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceDetail {
    #[schema(example = "66a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "default")]
    pub name: String,
    pub source_type: SourceType,
    #[schema(example = "https://api.riftl.ink/w/a1b2c3d4e5f6...")]
    pub webhook_url: String,
    #[schema(example = "2026-04-10T12:00:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSourcesResponse {
    pub sources: Vec<SourceDetail>,
}

/// Aggregated counts per `(link, type)` for embedding in `LinkStatsResponse`.
/// Computed on read from `conversion_events` via an aggregation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversionDetail {
    #[schema(example = "deposit")]
    pub conversion_type: String,
    #[schema(example = 19)]
    pub count: u64,
}

// ── Ingestion result (service layer output) ──

/// Outcome of processing a batch of parsed conversions in `ConversionsService::ingest_parsed`.
#[derive(Debug, Clone, Default)]
pub struct IngestResult {
    pub accepted: usize,
    pub deduped: usize,
    pub unattributed: usize,
    pub failed: usize,
}
