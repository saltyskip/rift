use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};

use super::models::{Attribution, Click, CreateLinkInput, Link};

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

    async fn list_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Link>, String>;

    async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
        token: Option<String>,
    ) -> Result<(), String>;

    async fn find_click_by_token(&self, token: &str) -> Result<Option<Click>, String>;

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String>;

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
}

// ── Repository ──

#[derive(Clone)]
pub struct LinksRepo {
    links: Collection<Link>,
    clicks: Collection<Click>,
    attributions: Collection<Attribution>,
}

macro_rules! ensure_index {
    ($col:expr, $keys:expr, $opts:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(IndexModel::builder().keys($keys).options($opts).build())
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
    ($col:expr, $keys:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(IndexModel::builder().keys($keys).build())
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
}

impl LinksRepo {
    pub async fn new(database: &Database) -> Self {
        let links = database.collection::<Link>("links");
        let clicks = database.collection::<Click>("clicks");
        let attributions = database.collection::<Attribution>("attributions");

        // Drop the old global unique index if it exists (replaced by compound index).
        let _ = links.drop_index("link_id_unique").await;

        ensure_index!(
            links,
            doc! { "tenant_id": 1, "link_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "tenant_link_id_unique"
        );

        ensure_index!(
            clicks,
            doc! { "tenant_id": 1, "link_id": 1 },
            "clicks_tenant_link"
        );

        ensure_index!(
            clicks,
            doc! { "token": 1 },
            IndexOptions::builder().unique(true).sparse(true).build(),
            "clicks_token_sparse_unique"
        );

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

        LinksRepo {
            links,
            clicks,
            attributions,
        }
    }
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
        };
        self.links
            .insert_one(&link)
            .await
            .map_err(|e| e.to_string())?;
        Ok(link)
    }

    async fn find_link_by_id(&self, link_id: &str) -> Result<Option<Link>, String> {
        self.links
            .find_one(doc! { "link_id": link_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_link_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Option<Link>, String> {
        self.links
            .find_one(doc! { "tenant_id": tenant_id, "link_id": link_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Link>, String> {
        let mut cursor = self
            .links
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .limit(100)
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
        token: Option<String>,
    ) -> Result<(), String> {
        let click = Click {
            id: ObjectId::new(),
            tenant_id,
            link_id: link_id.to_string(),
            clicked_at: DateTime::now(),
            user_agent,
            referer,
            platform,
            token,
        };
        self.clicks
            .insert_one(&click)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_click_by_token(&self, token: &str) -> Result<Option<Click>, String> {
        self.clicks
            .find_one(doc! { "token": token })
            .await
            .map_err(|e| e.to_string())
    }

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        self.clicks
            .count_documents(doc! { "tenant_id": tenant_id, "link_id": link_id })
            .await
            .map_err(|e| e.to_string())
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
}
