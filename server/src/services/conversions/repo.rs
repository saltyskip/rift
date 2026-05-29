use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::{IndexOptions, TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};
use rand::RngCore;

use crate::ensure_index;

use super::models::{ConversionDedup, ConversionEvent, Source, SourceType};
use crate::core::public_id::SourceId;
use crate::services::links::models::CreditModel;

/// Sentinel `source_id` for events that came in via the SDK direct endpoint
/// rather than a registered `Source`. Stored on `ConversionMeta.source_id` so
/// the field stays non-optional in the time series schema; downstream readers
/// treat it as "no upstream source row exists."
pub fn sdk_sentinel_source_id() -> SourceId {
    SourceId::from_object_id(ObjectId::from_bytes([0u8; 12]))
}

// ── Trait ──

#[async_trait]
pub trait ConversionsRepository: Send + Sync {
    // ── Sources CRUD ──

    async fn create_source(
        &self,
        tenant_id: ObjectId,
        name: String,
        source_type: SourceType,
    ) -> Result<Source, String>;

    async fn find_source_by_token(&self, url_token: &str) -> Result<Option<Source>, String>;

    async fn find_source_by_id(
        &self,
        tenant_id: &ObjectId,
        id: &ObjectId,
    ) -> Result<Option<Source>, String>;

    async fn list_sources(&self, tenant_id: &ObjectId) -> Result<Vec<Source>, String>;

    async fn delete_source(&self, tenant_id: &ObjectId, id: &ObjectId) -> Result<bool, String>;

    /// Returns the tenant's default custom source, creating it (name: "default")
    /// on first access. Enables the zero-ceremony dev flow of `GET /v1/sources`
    /// returning a usable webhook URL immediately.
    async fn get_or_create_default_custom_source(
        &self,
        tenant_id: ObjectId,
    ) -> Result<Source, String>;

    // ── Events ──

    /// Insert a conversion event into the time series collection.
    /// Returns the ObjectId of the inserted document (used as `event_id` in the
    /// outbound webhook payload for customer-side dedup).
    async fn insert_conversion_event(&self, event: ConversionEvent) -> Result<ObjectId, String>;

    /// Atomically attempt to record an idempotency key.
    /// - `Ok(true)` — newly inserted; caller should proceed with the event
    /// - `Ok(false)` — duplicate; caller should silently skip
    /// - `Err(_)` — other DB error; caller logs and skips
    async fn check_and_insert_dedup(
        &self,
        tenant_id: &ObjectId,
        idempotency_key: &str,
    ) -> Result<bool, String>;

    // ── Aggregations ──

    /// Count conversions per type for a set of user_ids within a time
    /// range. Returns one entry per non-zero conversion type.
    /// Empty input → empty output.
    #[allow(dead_code)] // kept for reference / future debug; funnel uses the credited variant
    async fn count_by_type_for_users(
        &self,
        tenant_id: &ObjectId,
        user_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<(String, u64)>, String>;

    /// Count conversions per type whose **credited link** is in
    /// `link_ids`, under the chosen attribution model. This is the
    /// correct funnel-side conversion count: a conversion is attributed
    /// to a campaign by walking the user's `attribution_events` chain
    /// up to (and including) `occurred_at`, picking the boundary event
    /// per the credit model, and checking whether its `link_id` is in
    /// the campaign set.
    ///
    /// Why not just count conversions for the campaign's installs'
    /// users? Because a multi-touch user (campaign A on phone, campaign
    /// B on tablet, then conversion) would land in both A's and B's
    /// funnel — double-credit. This method credits each conversion to
    /// exactly one campaign per credit model (or to any campaign it
    /// touched, for `Touched`).
    ///
    /// Returns an empty vec for empty `link_ids`.
    async fn count_conversions_by_type_credited_to_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
        credit: CreditModel,
    ) -> Result<Vec<(String, u64)>, String>;
}

// ── Repository ──

crate::impl_container!(ConversionsRepo);
#[derive(Clone)]
pub struct ConversionsRepo {
    sources: Collection<Source>,
    events: Collection<ConversionEvent>,
    dedup: Collection<ConversionDedup>,
}

impl ConversionsRepo {
    pub async fn new(database: &Database) -> Self {
        let sources = database.collection::<Source>("sources");

        // conversion_events is a time series collection — same pattern as click_events.
        let ts_opts = TimeseriesOptions::builder()
            .time_field("occurred_at".to_string())
            .meta_field(Some("meta".to_string()))
            .granularity(Some(TimeseriesGranularity::Minutes))
            .build();
        if let Err(e) = database
            .create_collection("conversion_events")
            .timeseries(ts_opts)
            .await
        {
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("48") {
                tracing::error!("Failed to create conversion_events time series collection: {e}");
            }
        }
        let events = database.collection::<ConversionEvent>("conversion_events");

        // Per-tier retention — same partial TTL pattern as click_events.
        crate::services::billing::retention::ensure_retention_ttl_indexes(
            &events,
            "occurred_at",
            "meta",
        )
        .await;

        let dedup = database.collection::<ConversionDedup>("conversion_dedup");

        // ── Indexes ──

        ensure_index!(
            sources,
            doc! { "tenant_id": 1, "name": 1 },
            IndexOptions::builder().unique(true).build(),
            "sources_tenant_name_unique"
        );
        ensure_index!(
            sources,
            doc! { "url_token": 1 },
            IndexOptions::builder().unique(true).build(),
            "sources_url_token_unique"
        );

        // Dedup: atomic uniqueness enforcement + TTL for automatic cleanup.
        ensure_index!(
            dedup,
            doc! { "tenant_id": 1, "idempotency_key": 1 },
            IndexOptions::builder().unique(true).build(),
            "dedup_tenant_key_unique"
        );
        ensure_index!(
            dedup,
            doc! { "created_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(30 * 24 * 60 * 60))
                .build(),
            "dedup_ttl_30d"
        );

        ConversionsRepo {
            sources,
            events,
            dedup,
        }
    }
}

fn generate_url_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[async_trait]
impl ConversionsRepository for ConversionsRepo {
    async fn create_source(
        &self,
        tenant_id: ObjectId,
        name: String,
        source_type: SourceType,
    ) -> Result<Source, String> {
        let doc = Source {
            id: crate::core::public_id::SourceId::new(),
            tenant_id: crate::core::public_id::TenantId::from_object_id(tenant_id),
            name,
            source_type,
            url_token: generate_url_token(),
            signing_secret: None,
            config: Document::new(),
            created_at: DateTime::now(),
        };
        self.sources
            .insert_one(&doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(doc)
    }

    async fn find_source_by_token(&self, url_token: &str) -> Result<Option<Source>, String> {
        self.sources
            .find_one(doc! { "url_token": url_token })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_source_by_id(
        &self,
        tenant_id: &ObjectId,
        id: &ObjectId,
    ) -> Result<Option<Source>, String> {
        self.sources
            .find_one(doc! { "_id": id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_sources(&self, tenant_id: &ObjectId) -> Result<Vec<Source>, String> {
        let mut cursor = self
            .sources
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut sources = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            sources.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(sources)
    }

    async fn delete_source(&self, tenant_id: &ObjectId, id: &ObjectId) -> Result<bool, String> {
        let result = self
            .sources
            .delete_one(doc! { "_id": id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }

    async fn get_or_create_default_custom_source(
        &self,
        tenant_id: ObjectId,
    ) -> Result<Source, String> {
        if let Some(existing) = self
            .sources
            .find_one(doc! { "tenant_id": &tenant_id, "name": "default" })
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(existing);
        }

        // Race-safe create: if another request beat us, the unique index kicks in
        // and we fall back to re-reading the existing row.
        match self
            .create_source(tenant_id, "default".to_string(), SourceType::Custom)
            .await
        {
            Ok(source) => Ok(source),
            Err(e) if e.contains("E11000") => self
                .sources
                .find_one(doc! { "tenant_id": &tenant_id, "name": "default" })
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "default source disappeared after conflict".to_string()),
            Err(e) => Err(e),
        }
    }

    async fn insert_conversion_event(&self, event: ConversionEvent) -> Result<ObjectId, String> {
        // Time series inserts do not return a generated _id in the same way as
        // regular collections, so synthesize one for the outbound webhook payload.
        // The ObjectId is stable for idempotency but does not correspond to a
        // persisted document _id in the time series collection.
        let synthetic_id = ObjectId::new();
        self.events
            .insert_one(&event)
            .await
            .map_err(|e| e.to_string())?;
        Ok(synthetic_id)
    }

    async fn check_and_insert_dedup(
        &self,
        tenant_id: &ObjectId,
        idempotency_key: &str,
    ) -> Result<bool, String> {
        let doc = ConversionDedup {
            id: crate::services::conversions::models::ConversionDedupId::new(),
            tenant_id: crate::core::public_id::TenantId::from_object_id(*tenant_id),
            idempotency_key: idempotency_key.to_string(),
            created_at: DateTime::now(),
        };
        match self.dedup.insert_one(&doc).await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("E11000") => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn count_by_type_for_users(
        &self,
        tenant_id: &ObjectId,
        user_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<(String, u64)>, String> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let bson_ids: Vec<mongodb::bson::Bson> = user_ids
            .iter()
            .map(|s| mongodb::bson::Bson::String(s.clone()))
            .collect();
        let pipeline = vec![
            doc! {
                "$match": {
                    "meta.tenant_id": tenant_id,
                    "user_id": { "$in": bson_ids },
                    "occurred_at": { "$gte": from, "$lte": to },
                }
            },
            doc! {
                "$group": {
                    "_id": "$meta.conversion_type",
                    "count": { "$sum": 1 },
                }
            },
        ];

        let mut cursor = self
            .events
            .aggregate(pipeline)
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            let raw: Document = cursor.deserialize_current().map_err(|e| e.to_string())?;
            let conversion_type = raw.get_str("_id").map_err(|e| e.to_string())?.to_string();
            let count = count_field_as_u64(&raw, "count");
            if count > 0 {
                results.push((conversion_type, count));
            }
        }
        Ok(results)
    }

    async fn count_conversions_by_type_credited_to_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
        credit: CreditModel,
    ) -> Result<Vec<(String, u64)>, String> {
        if link_ids.is_empty() {
            return Ok(Vec::new());
        }
        let bson_links: Vec<mongodb::bson::Bson> = link_ids
            .iter()
            .map(|s| mongodb::bson::Bson::String(s.clone()))
            .collect();

        // For first/last touch the inner pipeline walks the user's
        // attribution chain up to occurred_at, picks the boundary event,
        // and projects its link_id; the outer $match keeps the
        // conversion only when that boundary link is in our campaign
        // set. For touched, the inner pipeline filters to events whose
        // link is already in the set — any match credits the conversion
        // once.
        // Note: the inner pipeline runs against `attribution_events`,
        // where `user_id` lives under `meta` (so the identify backfill
        // can actually mutate it — Mongo time-series only updates
        // meta-field paths). The outer `$$uid` is the conversion
        // event's top-level user_id (insert-only there, no mutation
        // concern).
        let inner_pipeline: Vec<Document> = match credit {
            CreditModel::Touched => vec![
                doc! {
                    "$match": {
                        "$expr": {
                            "$and": [
                                { "$eq": ["$meta.tenant_id", tenant_id] },
                                { "$eq": ["$meta.user_id", "$$uid"] },
                                { "$lte": ["$timestamp", "$$convo_t"] },
                                { "$in": ["$link_id", bson_links.clone()] },
                            ]
                        }
                    }
                },
                doc! { "$limit": 1 },
                doc! { "$project": { "_id": 1 } },
            ],
            CreditModel::FirstTouch | CreditModel::LastTouch => {
                let sort_dir: i32 = if matches!(credit, CreditModel::FirstTouch) {
                    1
                } else {
                    -1
                };
                vec![
                    doc! {
                        "$match": {
                            "$expr": {
                                "$and": [
                                    { "$eq": ["$meta.tenant_id", tenant_id] },
                                    { "$eq": ["$meta.user_id", "$$uid"] },
                                    { "$lte": ["$timestamp", "$$convo_t"] },
                                ]
                            }
                        }
                    },
                    doc! { "$sort": { "timestamp": sort_dir } },
                    doc! { "$limit": 1 },
                    doc! { "$project": { "_id": 0, "link_id": 1 } },
                ]
            }
        };

        let post_lookup_match: Document = match credit {
            CreditModel::Touched => doc! {
                "$match": { "$expr": { "$gt": [{ "$size": "$credit" }, 0] } }
            },
            CreditModel::FirstTouch | CreditModel::LastTouch => doc! {
                "$match": { "credit.0.link_id": { "$in": bson_links } }
            },
        };

        let pipeline = vec![
            doc! {
                "$match": {
                    "meta.tenant_id": tenant_id,
                    "occurred_at": { "$gte": from, "$lte": to },
                    "user_id": { "$ne": null },
                }
            },
            doc! {
                "$lookup": {
                    "from": "attribution_events",
                    "let": { "uid": "$user_id", "convo_t": "$occurred_at" },
                    "pipeline": inner_pipeline,
                    "as": "credit",
                }
            },
            post_lookup_match,
            doc! {
                "$group": {
                    "_id": "$meta.conversion_type",
                    "count": { "$sum": 1 },
                }
            },
        ];

        let mut cursor = self
            .events
            .aggregate(pipeline)
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            let raw: Document = cursor.deserialize_current().map_err(|e| e.to_string())?;
            let conversion_type = raw.get_str("_id").map_err(|e| e.to_string())?.to_string();
            let count = count_field_as_u64(&raw, "count");
            if count > 0 {
                results.push((conversion_type, count));
            }
        }
        Ok(results)
    }
}

/// `$sum: 1` and `$count: {}` return BSON Int32 for small totals and
/// promote to Int64 only when needed — but `Document::get_i64` errors
/// on Int32 inputs. The previous `raw.get_i64("count").unwrap_or(0)`
/// silently dropped every group whose count fit in 32 bits (i.e. every
/// realistic value), and the row was filtered out by the `count > 0`
/// guard. This helper accepts either width and treats anything else as
/// zero.
fn count_field_as_u64(doc: &Document, key: &str) -> u64 {
    match doc.get(key) {
        Some(mongodb::bson::Bson::Int64(n)) => (*n).max(0) as u64,
        Some(mongodb::bson::Bson::Int32(n)) => (*n as i64).max(0) as u64,
        Some(mongodb::bson::Bson::Double(n)) => n.max(0.0) as u64,
        _ => 0,
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
