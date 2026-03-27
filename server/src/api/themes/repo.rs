use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::LandingTheme;

#[async_trait]
pub trait ThemesRepository: Send + Sync {
    async fn create_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String>;
    async fn list_by_tenant(
        &self,
        tenant_id: &ObjectId,
        status: Option<&str>,
    ) -> Result<Vec<LandingTheme>, String>;
    async fn find_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        theme_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String>;
    async fn find_default_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String>;
    async fn find_by_tenant_and_slug(
        &self,
        tenant_id: &ObjectId,
        slug: &str,
    ) -> Result<Option<LandingTheme>, String>;
    async fn replace_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String>;
    async fn clear_default_for_tenant(
        &self,
        tenant_id: &ObjectId,
        except_theme_id: Option<&ObjectId>,
    ) -> Result<(), String>;
    async fn delete_theme(&self, tenant_id: &ObjectId, theme_id: &ObjectId) -> Result<bool, String>;
}

#[derive(Clone)]
pub struct ThemesRepo {
    themes: Collection<LandingTheme>,
}

impl ThemesRepo {
    pub async fn new(database: &Database) -> Self {
        let themes = database.collection::<LandingTheme>("themes");
        ensure_index!(
            themes,
            doc! { "tenant_id": 1, "slug": 1 },
            IndexOptions::builder().unique(true).build(),
            "themes_tenant_slug_unique"
        );
        ensure_index!(themes, doc! { "tenant_id": 1, "is_default": 1 }, "themes_tenant_default");
        Self { themes }
    }
}

#[async_trait]
impl ThemesRepository for ThemesRepo {
    async fn create_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String> {
        self.themes.insert_one(&theme).await.map_err(|e| e.to_string())?;
        Ok(theme)
    }

    async fn list_by_tenant(
        &self,
        tenant_id: &ObjectId,
        status: Option<&str>,
    ) -> Result<Vec<LandingTheme>, String> {
        let mut filter = doc! { "tenant_id": tenant_id };
        if let Some(status) = status {
            filter.insert("status", status);
        }

        let mut cursor = self
            .themes
            .find(filter)
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut themes = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            themes.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(themes)
    }

    async fn find_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        theme_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String> {
        self.themes
            .find_one(doc! { "_id": theme_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_default_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String> {
        self.themes
            .find_one(doc! { "tenant_id": tenant_id, "is_default": true })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_tenant_and_slug(
        &self,
        tenant_id: &ObjectId,
        slug: &str,
    ) -> Result<Option<LandingTheme>, String> {
        self.themes
            .find_one(doc! { "tenant_id": tenant_id, "slug": slug })
            .await
            .map_err(|e| e.to_string())
    }

    async fn replace_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String> {
        self.themes
            .replace_one(doc! { "_id": &theme.id, "tenant_id": &theme.tenant_id }, &theme)
            .await
            .map_err(|e| e.to_string())?;
        Ok(theme)
    }

    async fn clear_default_for_tenant(
        &self,
        tenant_id: &ObjectId,
        except_theme_id: Option<&ObjectId>,
    ) -> Result<(), String> {
        let mut filter = doc! { "tenant_id": tenant_id, "is_default": true };
        if let Some(theme_id) = except_theme_id {
            filter.insert("_id", doc! { "$ne": theme_id });
        }

        self.themes
            .update_many(filter, doc! { "$set": { "is_default": false } })
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn delete_theme(&self, tenant_id: &ObjectId, theme_id: &ObjectId) -> Result<bool, String> {
        let result = self
            .themes
            .delete_one(doc! { "_id": theme_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }
}
