use async_trait::async_trait;
use cached::proc_macro::cached;
use cached::Cached;
use mongodb::bson::{self, doc, oid::ObjectId, DateTime};
use mongodb::options::{IndexOptions, TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};

use crate::ensure_index;

use mongodb::bson::Document;

use super::models::{
    AttributionEvent, AttributionEventMeta, BulkInsertError, ClickEvent, ClickMeta,
    CreateLinkInput, CreditModel, CreditedLinks, Link, LinkStatus,
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

    /// Append an immutable `/lifecycle/attribute` event to the
    /// `attribution_events` time-series collection. `user_id` is stamped
    /// into `meta.user_id` at write time when the install is already
    /// bound (resolved by the service via `app_users`) so user-anchored
    /// reads can prune buckets.
    async fn record_attribute_event(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
        user_id: Option<&str>,
        retention_bucket: String,
    ) -> Result<(), String>;

    /// Backfill `meta.user_id` onto every `attribution_events` row for
    /// this install. Called from the identify path so pre-identify
    /// anonymous events become user-anchored after the binding.
    ///
    /// `user_id` lives in `meta` (not a data field) specifically because
    /// MongoDB time-series only supports updates on meta-field paths.
    /// A prior incarnation of this method put `user_id` at the top level
    /// and the `update_many` silently no-opped on every call.
    ///
    /// Returns the number of rows updated.
    async fn backfill_user_id_on_attribution_events(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<u64, String>;

    /// Distinct install_ids credited to the given link set within the
    /// time range, under the chosen attribution model:
    ///
    /// - `Touched`:    install has ANY event with `link_id` in set
    /// - `FirstTouch`: install's FIRST event in range has `link_id` in set
    /// - `LastTouch`:  install's LAST event in range has `link_id` in set
    ///
    /// Returns an empty set if `link_ids` is empty.
    async fn distinct_install_ids_credited_to_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
        credit: CreditModel,
    ) -> Result<Vec<String>, String>;

    /// Count click events for a set of links in the time range.
    async fn count_clicks_for_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<u64, String>;

    /// Resolve the user's credited links at a moment in time.
    ///
    /// Walks the user's `attribution_events` with `timestamp ≤ at_or_before`
    /// and returns the first-touch and last-touch link_ids in one round
    /// trip. Both are returned so webhook payloads can carry both
    /// flavours — receivers pick whichever matches their attribution
    /// philosophy without needing to query Rift back.
    ///
    /// Returns `(None, None)` for a user with no attribution events on
    /// or before the cutoff (a backend-fired conversion for a user who
    /// hasn't done a `/lifecycle/attribute` yet, for example).
    async fn credited_links_for_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
        at_or_before: DateTime,
    ) -> Result<CreditedLinks, String>;
}

// ── Repository ──

crate::impl_container!(LinksRepo);
#[derive(Clone)]
pub struct LinksRepo {
    links: Collection<Link>,
    click_events: Collection<ClickEvent>,
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

        // Best-effort drop of the obsolete (meta.tenant_id, user_id,
        // timestamp) index. That filter shape never worked — Mongo
        // time-series rejected the `user_id` data-field filter clause
        // silently — and `user_id` now lives under `meta`, so the
        // index would never get used even if it had matched. Bucket
        // pruning on `meta.user_id` is handled implicitly by the
        // time-series storage.
        let _ = attribution_events
            .drop_index("attribution_events_tenant_user_time")
            .await;

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

        LinksRepo {
            links,
            click_events,
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
        invalidate_link_cache(link.tenant_id.as_object_id(), &link.link_id).await;
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
                    invalidate_link_cache(d.tenant_id.as_object_id(), &d.link_id).await;
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

    async fn record_attribute_event(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
        user_id: Option<&str>,
        retention_bucket: String,
    ) -> Result<(), String> {
        let event = AttributionEvent {
            timestamp: DateTime::now(),
            meta: AttributionEventMeta {
                tenant_id,
                install_id: install_id.to_string(),
                retention_bucket,
                user_id: user_id.map(|s| s.to_string()),
            },
            link_id: link_id.to_string(),
            app_version: app_version.to_string(),
        };
        self.attribution_events
            .insert_one(&event)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn backfill_user_id_on_attribution_events(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        // Filter AND update must target meta-field paths only —
        // MongoDB time-series silently no-ops if either touches a
        // data field. That's why `user_id` lives in `meta`. The
        // filter intentionally omits any "only fill nulls" clause:
        // an idempotent `$set` to the same value is a no-op at
        // bucket-rewrite time, and adding a data-field filter would
        // re-introduce the silent failure.
        let result = self
            .attribution_events
            .update_many(
                doc! {
                    "meta.tenant_id": tenant_id,
                    "meta.install_id": install_id,
                },
                doc! { "$set": { "meta.user_id": user_id } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.modified_count)
    }

    async fn distinct_install_ids_credited_to_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
        credit: CreditModel,
    ) -> Result<Vec<String>, String> {
        if link_ids.is_empty() {
            return Ok(Vec::new());
        }
        let bson_ids: Vec<bson::Bson> = link_ids
            .iter()
            .map(|s| bson::Bson::String(s.clone()))
            .collect();

        // `Touched` is a simple distinct over events with link_id in set.
        // `FirstTouch` / `LastTouch` require grouping by install and
        // picking the boundary event in the time range.
        match credit {
            CreditModel::Touched => {
                // `link_id` is a top-level field on AttributionEvent, not
                // in `meta` (meta = tenant_id + install_id +
                // retention_bucket only). A previous version of this
                // filter used `meta.link_id` and silently returned an
                // empty set on every Touched query.
                let values = self
                    .attribution_events
                    .distinct(
                        "meta.install_id",
                        doc! {
                            "meta.tenant_id": tenant_id,
                            "link_id": { "$in": bson_ids },
                            "timestamp": { "$gte": from, "$lte": to },
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(values
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect());
            }
            CreditModel::FirstTouch | CreditModel::LastTouch => {}
        }

        // Sort direction picks first vs last per install. $first inside
        // the group then captures the boundary event's link_id.
        let sort_dir: i32 = if matches!(credit, CreditModel::FirstTouch) {
            1
        } else {
            -1
        };
        let pipeline = vec![
            doc! {
                "$match": {
                    "meta.tenant_id": tenant_id,
                    "timestamp": { "$gte": from, "$lte": to },
                }
            },
            doc! { "$sort": { "meta.install_id": 1, "timestamp": sort_dir } },
            doc! {
                "$group": {
                    "_id": "$meta.install_id",
                    "boundary_link_id": { "$first": "$link_id" },
                }
            },
            doc! { "$match": { "boundary_link_id": { "$in": bson_ids } } },
            doc! { "$project": { "_id": 1 } },
        ];

        let mut cursor = self
            .attribution_events
            .aggregate(pipeline)
            .await
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            let raw: Document = cursor.deserialize_current().map_err(|e| e.to_string())?;
            if let Ok(s) = raw.get_str("_id") {
                out.push(s.to_string());
            }
        }
        Ok(out)
    }

    async fn count_clicks_for_links(
        &self,
        tenant_id: &ObjectId,
        link_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<u64, String> {
        if link_ids.is_empty() {
            return Ok(0);
        }
        let bson_ids: Vec<bson::Bson> = link_ids
            .iter()
            .map(|s| bson::Bson::String(s.clone()))
            .collect();
        self.click_events
            .count_documents(doc! {
                "meta.tenant_id": tenant_id,
                "meta.link_id": { "$in": bson_ids },
                "clicked_at": { "$gte": from, "$lte": to },
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn credited_links_for_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
        at_or_before: DateTime,
    ) -> Result<CreditedLinks, String> {
        // One aggregation, two boundaries via $facet. The filter is
        // entirely on meta-field paths so the time-series planner can
        // prune buckets; `$first` after a sort gives the boundary
        // event in O(1) hops once the planner picks the right walk
        // direction.
        let match_stage = doc! {
            "meta.tenant_id": tenant_id,
            "meta.user_id": user_id,
            "timestamp": { "$lte": at_or_before },
        };
        let pipeline = vec![
            doc! { "$match": match_stage },
            doc! {
                "$facet": {
                    "first": [
                        { "$sort": { "timestamp": 1 } },
                        { "$limit": 1 },
                        { "$project": { "_id": 0, "link_id": 1 } },
                    ],
                    "last": [
                        { "$sort": { "timestamp": -1 } },
                        { "$limit": 1 },
                        { "$project": { "_id": 0, "link_id": 1 } },
                    ],
                }
            },
        ];

        let mut cursor = self
            .attribution_events
            .aggregate(pipeline)
            .await
            .map_err(|e| e.to_string())?;

        if !cursor.advance().await.map_err(|e| e.to_string())? {
            return Ok(CreditedLinks::default());
        }
        let raw: Document = cursor.deserialize_current().map_err(|e| e.to_string())?;

        let extract = |key: &str| -> Option<String> {
            raw.get_array(key)
                .ok()?
                .first()?
                .as_document()?
                .get_str("link_id")
                .ok()
                .map(|s| s.to_string())
        };

        Ok(CreditedLinks {
            first_touch_link_id: extract("first"),
            last_touch_link_id: extract("last"),
            // Metadata is enriched in the service layer (cached
            // `find_link_by_tenant_and_id`) — the repo returns IDs only.
            first_touch_link_metadata: None,
            last_touch_link_metadata: None,
        })
    }
}

fn build_link(input: CreateLinkInput) -> Link {
    Link {
        id: crate::core::public_id::LinkInternalId::new(),
        tenant_id: crate::core::public_id::TenantId::from_object_id(input.tenant_id),
        link_id: input.link_id,
        ios_deep_link: input.ios_deep_link,
        android_deep_link: input.android_deep_link,
        web_url: input.web_url,
        ios_store_url: input.ios_store_url,
        android_store_url: input.android_store_url,
        metadata: input.metadata,
        affiliate_id: input
            .affiliate_id
            .map(crate::core::public_id::AffiliateId::from_object_id),
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
