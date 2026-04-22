use async_trait::async_trait;
use cached::proc_macro::cached;
use cached::Cached;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::{IndexOptions, TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};

use crate::ensure_index;

use mongodb::bson::Document;

use super::models::{
    Attribution, ClickEvent, ClickMeta, CreateLinkInput, Link, LinkStatus, TimeseriesDataPoint,
};

// ── Trait ──

#[async_trait]
pub trait LinksRepository: Send + Sync {
    async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String>;

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
    ) -> Result<(), String>;

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String>;

    async fn get_click_timeseries(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<TimeseriesDataPoint>, String>;

    async fn upsert_attribution(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
    ) -> Result<(), String>;

    async fn link_attribution_to_user(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<bool, String>;

    async fn count_attributions(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String>;

    /// Find the Attribution record for a given `user_id` within a tenant. Used
    /// by the conversion ingestion path to resolve `user_id → link_id` so events
    /// can be attributed back to the link that drove the install.
    async fn find_attribution_by_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<Attribution>, String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct LinksRepo {
    links: Collection<Link>,
    click_events: Collection<ClickEvent>,
    attributions: Collection<Attribution>,
}

impl LinksRepo {
    pub async fn new(database: &Database) -> Self {
        let links = database.collection::<Link>("links");
        let attributions = database.collection::<Attribution>("attributions");

        // Create time series collection for click events (idempotent — errors if exists).
        let ts_opts = TimeseriesOptions::builder()
            .time_field("clicked_at".to_string())
            .meta_field(Some("meta".to_string()))
            .granularity(Some(TimeseriesGranularity::Minutes))
            .build();
        if let Err(e) = database
            .create_collection("click_events")
            .timeseries(ts_opts)
            .await
        {
            // NamespaceExists (code 48) is expected on subsequent startups.
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("48") {
                tracing::error!("Failed to create click_events time series collection: {e}");
            }
        }
        let click_events = database.collection::<ClickEvent>("click_events");

        // Drop the old global unique index if it exists (replaced by compound index).
        let _ = links.drop_index("link_id_unique").await;

        ensure_index!(
            links,
            doc! { "tenant_id": 1, "link_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "tenant_link_id_unique"
        );

        // Non-unique index on link_id for public resolution via /r/{link_id}.
        ensure_index!(links, doc! { "link_id": 1 }, "link_id_lookup");

        // The default _id index covers cursor-based pagination (sorted by _id desc).
        // ObjectIds are monotonically increasing, so _id order matches creation order.

        ensure_index!(
            attributions,
            doc! { "tenant_id": 1, "install_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "attr_tenant_install_unique"
        );
        ensure_index!(
            attributions,
            doc! { "tenant_id": 1, "link_id": 1 },
            "attr_tenant_link"
        );
        ensure_index!(
            attributions,
            doc! { "tenant_id": 1, "user_id": 1 },
            "attr_tenant_user"
        );

        LinksRepo {
            links,
            click_events,
            attributions,
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
        let link = Link {
            id: ObjectId::new(),
            tenant_id: input.tenant_id,
            link_id: input.link_id,
            ios_deep_link: input.ios_deep_link,
            android_deep_link: input.android_deep_link,
            web_url: input.web_url,
            ios_store_url: input.ios_store_url,
            android_store_url: input.android_store_url,
            metadata: input.metadata,
            created_at: DateTime::now(),
            status: LinkStatus::Active,
            flag_reason: None,
            expires_at: input.expires_at,
            agent_context: input.agent_context,
            social_preview: input.social_preview,
        };
        self.links
            .insert_one(&link)
            .await
            .map_err(|e| e.to_string())?;
        invalidate_link_cache(&link.tenant_id, &link.link_id).await;
        Ok(link)
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
    ) -> Result<(), String> {
        let event = ClickEvent {
            meta: ClickMeta {
                tenant_id,
                link_id: link_id.to_string(),
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

    async fn upsert_attribution(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
    ) -> Result<(), String> {
        self.attributions
            .update_one(
                doc! { "tenant_id": &tenant_id, "install_id": install_id },
                doc! {
                    "$setOnInsert": {
                        "_id": ObjectId::new(),
                        "tenant_id": &tenant_id,
                        "link_id": link_id,
                        "install_id": install_id,
                        "app_version": app_version,
                        "attributed_at": DateTime::now(),
                    }
                },
            )
            .upsert(true)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn link_attribution_to_user(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<bool, String> {
        let result = self
            .attributions
            .update_one(
                doc! {
                    "tenant_id": tenant_id,
                    "install_id": install_id,
                    "$or": [
                        { "user_id": { "$exists": false } },
                        { "user_id": null },
                        { "user_id": user_id },
                    ]
                },
                doc! { "$set": { "user_id": user_id } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.modified_count > 0 || result.matched_count > 0)
    }

    async fn count_attributions(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        self.attributions
            .count_documents(doc! { "tenant_id": tenant_id, "link_id": link_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_attribution_by_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<Attribution>, String> {
        self.attributions
            .find_one(doc! { "tenant_id": tenant_id, "user_id": user_id })
            .await
            .map_err(|e| e.to_string())
    }
}
