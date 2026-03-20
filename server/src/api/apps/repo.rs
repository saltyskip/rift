use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};

use super::models::App;

// ── Trait ──

#[async_trait]
pub trait AppsRepository: Send + Sync {
    async fn create_or_update(&self, app: App) -> Result<App, String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<App>, String>;
    async fn find_by_tenant_platform(
        &self,
        tenant_id: &ObjectId,
        platform: &str,
    ) -> Result<Option<App>, String>;
    async fn delete_app(&self, tenant_id: &ObjectId, app_id: &ObjectId) -> Result<bool, String>;
}

// ── Repository ──

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

#[derive(Clone)]
pub struct AppsRepo {
    apps: Collection<App>,
}

impl AppsRepo {
    pub async fn new(database: &Database) -> Self {
        let apps = database.collection::<App>("apps");

        ensure_index!(
            apps,
            doc! { "tenant_id": 1, "platform": 1 },
            IndexOptions::builder().unique(true).build(),
            "apps_tenant_platform_unique"
        );

        AppsRepo { apps }
    }
}

#[async_trait]
impl AppsRepository for AppsRepo {
    async fn create_or_update(&self, app: App) -> Result<App, String> {
        self.apps
            .update_one(
                doc! { "tenant_id": &app.tenant_id, "platform": &app.platform },
                doc! {
                    "$set": {
                        "bundle_id": &app.bundle_id,
                        "team_id": &app.team_id,
                        "package_name": &app.package_name,
                        "sha256_fingerprints": &app.sha256_fingerprints,
                        "app_name": &app.app_name,
                        "icon_url": &app.icon_url,
                        "theme_color": &app.theme_color,
                    },
                    "$setOnInsert": {
                        "_id": &app.id,
                        "tenant_id": &app.tenant_id,
                        "platform": &app.platform,
                        "created_at": DateTime::now(),
                    }
                },
            )
            .upsert(true)
            .await
            .map_err(|e| e.to_string())?;

        // Re-fetch so we return the actual document (correct _id and created_at).
        self.find_by_tenant_platform(&app.tenant_id, &app.platform)
            .await?
            .ok_or_else(|| "App not found after upsert".to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<App>, String> {
        let mut cursor = self
            .apps
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut apps = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            apps.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(apps)
    }

    async fn find_by_tenant_platform(
        &self,
        tenant_id: &ObjectId,
        platform: &str,
    ) -> Result<Option<App>, String> {
        self.apps
            .find_one(doc! { "tenant_id": tenant_id, "platform": platform })
            .await
            .map_err(|e| e.to_string())
    }

    async fn delete_app(&self, tenant_id: &ObjectId, app_id: &ObjectId) -> Result<bool, String> {
        let result = self
            .apps
            .delete_one(doc! { "_id": app_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }
}
