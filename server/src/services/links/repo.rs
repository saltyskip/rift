use async_trait::async_trait;
use cached::proc_macro::cached;
use cached::Cached;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::{IndexOptions, TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};

use crate::ensure_index;

use mongodb::bson::Document;

use super::models::{
    AttributeOutcome, AttributionEvent, AttributionEventMeta, BulkInsertError, ClickEvent,
    ClickMeta, CreateLinkInput, IdentifyOutcome, Install, Link, LinkStatus, TimeseriesDataPoint,
};

// ── Trait ──

#[async_trait]
pub trait LinksRepository: Send + Sync {
    async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String>;

    /// Insert a batch of links atomically. Either every input becomes a row
    /// or none do — the Mongo impl wraps the insert in a transaction so a
    /// race with a concurrent single-create that takes one of our ids
    /// surfaces as `DuplicateLinkIds(indices)` after rollback, never as a
    /// partial success.
    async fn create_many_in_txn(
        &self,
        inputs: Vec<CreateLinkInput>,
    ) -> Result<Vec<Link>, BulkInsertError>;

    async fn find_link_by_id(&self, link_id: &str) -> Result<Option<Link>, String>;

    async fn find_link_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Option<Link>, String>;

    async fn update_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        set: Document,
        unset: Document,
    ) -> Result<bool, String>;

    async fn delete_link(&self, tenant_id: &ObjectId, link_id: &str) -> Result<bool, String>;

    /// Total active links owned by this tenant — feeds the CreateLink quota.
    async fn count_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String>;

    async fn list_links_by_tenant(
        &self,
        tenant_id: &ObjectId,
        limit: i64,
        cursor: Option<ObjectId>,
    ) -> Result<Vec<Link>, String>;

    async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
        retention_bucket: String,
    ) -> Result<(), String>;

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String>;

    async fn get_click_timeseries(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<TimeseriesDataPoint>, String>;

    /// Record a `/lifecycle/attribute` call. Appends an immutable event to
    /// the `attribution_events` time-series collection AND upserts the
    /// install's first-touch state in `installs`. `installs.first_link_id`
    /// is set only on insert, so subsequent `attribute` calls for the same
    /// install append to the event log but preserve the original
    /// first-touch attribution.
    ///
    /// Returns `FirstTouch` on insert (`installs` row didn't exist), or
    /// `Retouch` on every subsequent call. Both carry the install row
    /// (with `user_id` if already bound) so the caller can build the
    /// outbound webhook payload without a second query.
    async fn record_attribute_event(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
        retention_bucket: String,
    ) -> Result<AttributeOutcome, String>;

    /// Bind `user_id` onto the install's row in `installs`. Idempotent
    /// at the same `(install_id, user_id)` pair (returns `AlreadyBound`).
    async fn identify_install(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<IdentifyOutcome, String>;

    /// Count distinct installs first-attributed to this link.
    async fn count_installs_by_first_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<u64, String>;

    /// Count installs first-attributed to this link whose `user_id` is
    /// bound — i.e. installs that progressed through
    /// `PUT /v1/lifecycle/identify`. `identify_count` in the stats
    /// response.
    async fn count_identifies(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String>;

    /// Find the install for a given `user_id` within a tenant. Used by
    /// the conversion ingestion path to resolve `user_id → first_link_id`
    /// so events can be attributed back to the link that drove the
    /// install.
    async fn find_install_by_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<Install>, String>;
}

// ── Repository ──

crate::impl_container!(LinksRepo);
#[derive(Clone)]
pub struct LinksRepo {
    links: Collection<Link>,
    click_events: Collection<ClickEvent>,
    /// Materialized first-touch + user_id binding per `(tenant_id,
    /// install_id)`. Mutable. Source of truth for stats counts and
    /// `user_id → link_id` conversion lookups.
    installs: Collection<Install>,
    /// Immutable time-series log of every `/lifecycle/attribute` call.
    /// Time-bucketed analytics + tier-based retention; never mutated.
    attribution_events: Collection<AttributionEvent>,
}

impl LinksRepo {
    pub async fn new(database: &Database) -> Self {
        let links = database.collection::<Link>("links");

        // Click events — time series, minute granularity.
        let click_ts_opts = TimeseriesOptions::builder()
            .time_field("clicked_at".to_string())
            .meta_field(Some("meta".to_string()))
            .granularity(Some(TimeseriesGranularity::Minutes))
            .build();
        if let Err(e) = database
            .create_collection("click_events")
            .timeseries(click_ts_opts)
            .await
        {
            // NamespaceExists (code 48) is expected on subsequent startups.
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("48") {
                tracing::error!("Failed to create click_events time series collection: {e}");
            }
        }
        let click_events = database.collection::<ClickEvent>("click_events");

        // Attribution events — time series, minute granularity. Same
        // meta-field convention as click_events so the retention TTL
        // helper applies unchanged.
        let attr_ts_opts = TimeseriesOptions::builder()
            .time_field("timestamp".to_string())
            .meta_field(Some("meta".to_string()))
            .granularity(Some(TimeseriesGranularity::Minutes))
            .build();
        if let Err(e) = database
            .create_collection("attribution_events")
            .timeseries(attr_ts_opts)
            .await
        {
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("48") {
                tracing::error!("Failed to create attribution_events time series collection: {e}");
            }
        }
        let attribution_events = database.collection::<AttributionEvent>("attribution_events");

        // Per-tier retention via partial TTL indexes on the timeField for
        // both time-series collections. Events are stamped at insert and
        // keep their bucket forever — tier downgrades don't retroactively
        // shrink historical retention.
        crate::services::billing::retention::ensure_retention_ttl_indexes(
            &click_events,
            "clicked_at",
            "meta",
        )
        .await;
        crate::services::billing::retention::ensure_retention_ttl_indexes(
            &attribution_events,
            "timestamp",
            "meta",
        )
        .await;

        // `installs` is a regular (non-time-series) collection because we
        // need to mutate `user_id` after insert — time-series rows are
        // immutable. The time-series `attribution_events` is the
        // append-only event log; `installs` is the mutable projection.
        let installs = database.collection::<Install>("installs");

        // Drop the old global unique index if it exists (replaced by compound index).
        let _ = links.drop_index("link_id_unique").await;

        ensure_index!(
            links,
            doc! { "tenant_id": 1, "link_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "tenant_link_id_unique"
        );
        ensure_index!(links, doc! { "link_id": 1 }, "link_id_lookup");
        ensure_index!(
            links,
            doc! { "tenant_id": 1, "affiliate_id": 1 },
            "links_tenant_affiliate"
        );

        ensure_index!(
            installs,
            doc! { "tenant_id": 1, "install_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "installs_tenant_install_unique"
        );
        // Stats: count distinct installs first-attributed to a link.
        ensure_index!(
            installs,
            doc! { "tenant_id": 1, "first_link_id": 1 },
            "installs_tenant_first_link"
        );
        // Conversion lookup: user_id → install (→ first_link_id).
        ensure_index!(
            installs,
            doc! { "tenant_id": 1, "user_id": 1 },
            "installs_tenant_user"
        );

        LinksRepo {
            links,
            click_events,
            installs,
            attribution_events,
        }
    }
}

// ── Cached lookups (1-hour TTL, max 50 000 entries) ──
//
// These return Err("not_found") on cache miss so that only Ok(Link) values
// are cached. The `#[cached(result = true)]` macro only caches Ok results,
// so Err (including not_found) is always re-executed. This prevents stale
// None entries from being served after a link is created.

const NOT_FOUND: &str = "not_found";

#[cached(
    ty = "cached::TimedSizedCache<String, Link>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(50_000, 3600) }",
    convert = r#"{ link_id.to_string() }"#,
    result = true
)]
async fn cached_find_link_by_id(links: &Collection<Link>, link_id: &str) -> Result<Link, String> {
    links
        .find_one(doc! { "link_id": link_id })
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| NOT_FOUND.to_string())
}

#[cached(
    ty = "cached::TimedSizedCache<String, Link>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(50_000, 3600) }",
    convert = r#"{ format!("{}:{}", tenant_id, link_id) }"#,
    result = true
)]
async fn cached_find_link_by_tenant_and_id(
    links: &Collection<Link>,
    tenant_id: &ObjectId,
    link_id: &str,
) -> Result<Link, String> {
    links
        .find_one(doc! { "tenant_id": tenant_id, "link_id": link_id })
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| NOT_FOUND.to_string())
}

/// Evict a link from both caches after a write (create/update/delete).
async fn invalidate_link_cache(tenant_id: &ObjectId, link_id: &str) {
    CACHED_FIND_LINK_BY_ID
        .lock()
        .await
        .cache_remove(&link_id.to_string());
    CACHED_FIND_LINK_BY_TENANT_AND_ID
        .lock()
        .await
        .cache_remove(&format!("{tenant_id}:{link_id}"));
}

#[async_trait]
impl LinksRepository for LinksRepo {
    async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String> {
        let link = build_link(input);
        self.links
            .insert_one(&link)
            .await
            .map_err(|e| e.to_string())?;
        invalidate_link_cache(&link.tenant_id, &link.link_id).await;
        Ok(link)
    }

    async fn create_many_in_txn(
        &self,
        inputs: Vec<CreateLinkInput>,
    ) -> Result<Vec<Link>, BulkInsertError> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }
        let docs: Vec<Link> = inputs.into_iter().map(build_link).collect();

        let mut session = self
            .links
            .client()
            .start_session()
            .await
            .map_err(|e| BulkInsertError::Internal(e.to_string()))?;

        session
            .start_transaction()
            .await
            .map_err(|e| BulkInsertError::Internal(e.to_string()))?;

        let result = self.links.insert_many(&docs).session(&mut session).await;

        match result {
            Ok(_) => {
                session
                    .commit_transaction()
                    .await
                    .map_err(|e| BulkInsertError::Internal(e.to_string()))?;
                for d in &docs {
                    invalidate_link_cache(&d.tenant_id, &d.link_id).await;
                }
                Ok(docs)
            }
            Err(e) => {
                let _ = session.abort_transaction().await;
                let dup_indices = parse_duplicate_indices(&e);
                if !dup_indices.is_empty() {
                    Err(BulkInsertError::DuplicateLinkIds(dup_indices))
                } else {
                    Err(BulkInsertError::Internal(e.to_string()))
                }
            }
        }
    }

    async fn find_link_by_id(&self, link_id: &str) -> Result<Option<Link>, String> {
        match cached_find_link_by_id(&self.links, link_id).await {
            Ok(link) => Ok(Some(link)),
            Err(e) if e == NOT_FOUND => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn find_link_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Option<Link>, String> {
        match cached_find_link_by_tenant_and_id(&self.links, tenant_id, link_id).await {
            Ok(link) => Ok(Some(link)),
            Err(e) if e == NOT_FOUND => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn update_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        set: Document,
        unset: Document,
    ) -> Result<bool, String> {
        let mut update_doc = Document::new();
        if !set.is_empty() {
            update_doc.insert("$set", set);
        }
        if !unset.is_empty() {
            update_doc.insert("$unset", unset);
        }
        let result = self
            .links
            .update_one(
                doc! { "tenant_id": tenant_id, "link_id": link_id },
                update_doc,
            )
            .await
            .map_err(|e| e.to_string())?;
        if result.matched_count > 0 {
            invalidate_link_cache(tenant_id, link_id).await;
        }
        Ok(result.matched_count > 0)
    }

    async fn delete_link(&self, tenant_id: &ObjectId, link_id: &str) -> Result<bool, String> {
        let result = self
            .links
            .delete_one(doc! { "tenant_id": tenant_id, "link_id": link_id })
            .await
            .map_err(|e| e.to_string())?;
        if result.deleted_count > 0 {
            invalidate_link_cache(tenant_id, link_id).await;
        }
        Ok(result.deleted_count > 0)
    }

    async fn count_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        self.links
            .count_documents(doc! { "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_links_by_tenant(
        &self,
        tenant_id: &ObjectId,
        limit: i64,
        cursor: Option<ObjectId>,
    ) -> Result<Vec<Link>, String> {
        let mut filter = doc! { "tenant_id": tenant_id };
        if let Some(cursor_id) = cursor {
            filter.insert("_id", doc! { "$lt": cursor_id });
        }

        let mut cursor = self
            .links
            .find(filter)
            .sort(doc! { "_id": -1 })
            .limit(limit)
            .await
            .map_err(|e| e.to_string())?;

        let mut links = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            links.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(links)
    }

    async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
        retention_bucket: String,
    ) -> Result<(), String> {
        let event = ClickEvent {
            meta: ClickMeta {
                tenant_id,
                link_id: link_id.to_string(),
                retention_bucket,
            },
            clicked_at: DateTime::now(),
            user_agent,
            referer,
            platform,
        };
        self.click_events
            .insert_one(&event)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        self.click_events
            .count_documents(doc! { "meta.tenant_id": tenant_id, "meta.link_id": link_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_click_timeseries(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<TimeseriesDataPoint>, String> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "meta.tenant_id": tenant_id,
                    "meta.link_id": link_id,
                    "clicked_at": { "$gte": from, "$lte": to }
                }
            },
            doc! {
                "$group": {
                    "_id": {
                        "$dateToString": { "format": "%Y-%m-%d", "date": "$clicked_at" }
                    },
                    "clicks": { "$sum": 1 }
                }
            },
            doc! { "$sort": { "_id": 1 } },
            doc! {
                "$project": {
                    "_id": 0,
                    "date": "$_id",
                    "clicks": 1
                }
            },
        ];

        let mut cursor = self
            .click_events
            .aggregate(pipeline)
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            let doc = cursor.deserialize_current().map_err(|e| e.to_string())?;
            let date = doc.get_str("date").map_err(|e| e.to_string())?.to_string();
            let clicks = doc
                .get_i64("clicks")
                .or_else(|_| doc.get_i32("clicks").map(|v| v as i64))
                .map_err(|e| e.to_string())? as u64;
            results.push(TimeseriesDataPoint { date, clicks });
        }
        Ok(results)
    }

    async fn record_attribute_event(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
        retention_bucket: String,
    ) -> Result<AttributeOutcome, String> {
        // Step 1: try to insert a fresh install row. `$setOnInsert` keeps
        // first-touch attribution stable across re-attributions. On insert
        // we capture `installs.first_link_id = link_id`; on no-op (row
        // already exists) the existing first_link_id stays.
        use mongodb::options::ReturnDocument;
        let now = DateTime::now();

        let install = self
            .installs
            .find_one_and_update(
                doc! { "tenant_id": &tenant_id, "install_id": install_id },
                doc! {
                    "$setOnInsert": {
                        "_id": ObjectId::new(),
                        "tenant_id": &tenant_id,
                        "install_id": install_id,
                        "first_link_id": link_id,
                        "first_app_version": app_version,
                        "first_attributed_at": now,
                    }
                },
            )
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await
            .map_err(|e| e.to_string())?;

        let is_first_touch = install.is_none();
        // Resolved post-update install. On first-touch we construct it
        // in-memory rather than re-query; on retouch we use the Before doc
        // (it's unchanged because $setOnInsert is a no-op).
        let install = install.unwrap_or_else(|| Install {
            id: ObjectId::new(), // not authoritative — we never use this in retouch path
            tenant_id,
            install_id: install_id.to_string(),
            first_link_id: link_id.to_string(),
            first_app_version: app_version.to_string(),
            first_attributed_at: now,
            user_id: None,
            identified_at: None,
        });

        // Step 2: append an immutable event to the time series. Includes
        // the install's current user_id so downstream subscribers can
        // act on existing-install re-attribution without a second roundtrip.
        let event = AttributionEvent {
            timestamp: now,
            meta: AttributionEventMeta {
                tenant_id,
                install_id: install_id.to_string(),
                retention_bucket,
            },
            link_id: link_id.to_string(),
            app_version: app_version.to_string(),
            user_id: install.user_id.clone(),
        };
        self.attribution_events
            .insert_one(&event)
            .await
            .map_err(|e| e.to_string())?;

        Ok(if is_first_touch {
            AttributeOutcome::FirstTouch(install)
        } else {
            AttributeOutcome::Retouch(install)
        })
    }

    async fn identify_install(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<IdentifyOutcome, String> {
        // `find_one_and_update` with `ReturnDocument::Before` lets us
        // distinguish a new bind from an idempotent rebind in a single
        // roundtrip: if `before.user_id == Some(user_id)`, this was a no-op
        // and the webhook should NOT fire.
        use mongodb::options::ReturnDocument;
        let before = self
            .installs
            .find_one_and_update(
                doc! {
                    "tenant_id": tenant_id,
                    "install_id": install_id,
                    "$or": [
                        { "user_id": { "$exists": false } },
                        { "user_id": null },
                        { "user_id": user_id },
                    ]
                },
                doc! { "$set": { "user_id": user_id, "identified_at": DateTime::now() } },
            )
            .return_document(ReturnDocument::Before)
            .await
            .map_err(|e| e.to_string())?;

        let Some(mut install) = before else {
            return Ok(IdentifyOutcome::NotFound);
        };

        let was_already_bound = install.user_id.as_deref() == Some(user_id);
        install.user_id = Some(user_id.to_string());
        install.identified_at = Some(DateTime::now());

        Ok(if was_already_bound {
            IdentifyOutcome::AlreadyBound(install)
        } else {
            IdentifyOutcome::NewBind(install)
        })
    }

    async fn count_installs_by_first_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<u64, String> {
        self.installs
            .count_documents(doc! { "tenant_id": tenant_id, "first_link_id": link_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn count_identifies(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        // Identify-completed installs first-attributed to this link.
        self.installs
            .count_documents(doc! {
                "tenant_id": tenant_id,
                "first_link_id": link_id,
                "user_id": { "$ne": null },
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_install_by_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<Install>, String> {
        self.installs
            .find_one(doc! { "tenant_id": tenant_id, "user_id": user_id })
            .await
            .map_err(|e| e.to_string())
    }
}

fn build_link(input: CreateLinkInput) -> Link {
    Link {
        id: ObjectId::new(),
        tenant_id: input.tenant_id,
        link_id: input.link_id,
        ios_deep_link: input.ios_deep_link,
        android_deep_link: input.android_deep_link,
        web_url: input.web_url,
        ios_store_url: input.ios_store_url,
        android_store_url: input.android_store_url,
        metadata: input.metadata,
        affiliate_id: input.affiliate_id,
        created_at: DateTime::now(),
        status: LinkStatus::Active,
        flag_reason: None,
        expires_at: input.expires_at,
        agent_context: input.agent_context,
        social_preview: input.social_preview,
    }
}

/// Map a Mongo `insert_many` failure to the input indices that hit a
/// duplicate-key (E11000). Returns an empty vec for any other error so the
/// caller can surface it as `Internal`.
fn parse_duplicate_indices(err: &mongodb::error::Error) -> Vec<usize> {
    use mongodb::error::ErrorKind;
    if let ErrorKind::InsertMany(insert_many_error) = &*err.kind {
        if let Some(write_errors) = &insert_many_error.write_errors {
            return write_errors
                .iter()
                .filter(|e| e.code == 11000)
                .map(|e| e.index)
                .collect();
        }
    }
    Vec::new()
}
