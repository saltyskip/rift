use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::{IndexOptions, TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};
use rand::RngCore;

use crate::ensure_index;

use super::models::{ConversionDedup, ConversionDetail, ConversionEvent, Source, SourceType};

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

    /// Count and sum conversions for a link, grouped by `(conversion_type, currency)`.
    /// Used by the link stats endpoint.
    async fn get_conversion_counts_for_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Vec<ConversionDetail>, String>;
}

// ── Repository ──

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
            id: ObjectId::new(),
            tenant_id,
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
            id: ObjectId::new(),
            tenant_id: *tenant_id,
            idempotency_key: idempotency_key.to_string(),
            created_at: DateTime::now(),
        };
        match self.dedup.insert_one(&doc).await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("E11000") => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn get_conversion_counts_for_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Vec<ConversionDetail>, String> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "meta.tenant_id": tenant_id,
                    "meta.link_id": link_id,
                }
            },
            doc! {
                "$group": {
                    "_id": {
                        "type": "$meta.conversion_type",
                        "currency": "$currency",
                    },
                    "count": { "$sum": 1 },
                    "sum_cents": {
                        "$sum": { "$ifNull": ["$amount_cents", 0i64] }
                    },
                    "has_amount": {
                        "$max": { "$cond": [{ "$ne": ["$amount_cents", null] }, 1, 0] }
                    },
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

            let id_doc = raw.get_document("_id").map_err(|e| e.to_string())?;
            let conversion_type = id_doc
                .get_str("type")
                .map_err(|e| e.to_string())?
                .to_string();
            let currency = id_doc.get_str("currency").ok().map(|s| s.to_string());
            let count = raw.get_i64("count").unwrap_or(0).max(0) as u64;
            let has_amount = raw.get_i32("has_amount").unwrap_or(0) != 0;

            let sum_cents = if has_amount {
                Some(raw.get_i64("sum_cents").unwrap_or(0))
            } else {
                None
            };

            results.push(ConversionDetail {
                conversion_type,
                count,
                sum_cents,
                currency: if has_amount { currency } else { None },
            });
        }

        Ok(results)
    }
}
