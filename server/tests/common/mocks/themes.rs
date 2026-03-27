use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::api::themes::models::{LandingTheme, ThemeStatus};
use rift::api::themes::repo::ThemesRepository;

#[derive(Default)]
pub struct MockThemesRepo {
    pub themes: Mutex<Vec<LandingTheme>>,
}

#[async_trait]
impl ThemesRepository for MockThemesRepo {
    async fn create_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String> {
        let mut themes = self.themes.lock().unwrap();
        if themes
            .iter()
            .any(|existing| existing.tenant_id == theme.tenant_id && existing.slug == theme.slug)
        {
            return Err("E11000 duplicate key".to_string());
        }
        themes.push(theme.clone());
        Ok(theme)
    }

    async fn list_by_tenant(
        &self,
        tenant_id: &ObjectId,
        status: Option<&str>,
    ) -> Result<Vec<LandingTheme>, String> {
        Ok(self
            .themes
            .lock()
            .unwrap()
            .iter()
            .filter(|theme| &theme.tenant_id == tenant_id)
            .filter(|theme| match status {
                Some("active") => matches!(theme.status, ThemeStatus::Active),
                Some("archived") => matches!(theme.status, ThemeStatus::Archived),
                _ => true,
            })
            .cloned()
            .collect())
    }

    async fn find_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        theme_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String> {
        Ok(self
            .themes
            .lock()
            .unwrap()
            .iter()
            .find(|theme| &theme.tenant_id == tenant_id && &theme.id == theme_id)
            .cloned())
    }

    async fn find_default_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<LandingTheme>, String> {
        Ok(self
            .themes
            .lock()
            .unwrap()
            .iter()
            .find(|theme| &theme.tenant_id == tenant_id && theme.is_default)
            .cloned())
    }

    async fn find_by_tenant_and_slug(
        &self,
        tenant_id: &ObjectId,
        slug: &str,
    ) -> Result<Option<LandingTheme>, String> {
        Ok(self
            .themes
            .lock()
            .unwrap()
            .iter()
            .find(|theme| &theme.tenant_id == tenant_id && theme.slug == slug)
            .cloned())
    }

    async fn replace_theme(&self, theme: LandingTheme) -> Result<LandingTheme, String> {
        let mut themes = self.themes.lock().unwrap();
        let Some(existing) = themes
            .iter_mut()
            .find(|existing| existing.tenant_id == theme.tenant_id && existing.id == theme.id)
        else {
            return Err("not_found".to_string());
        };
        *existing = theme.clone();
        Ok(theme)
    }

    async fn clear_default_for_tenant(
        &self,
        tenant_id: &ObjectId,
        except_theme_id: Option<&ObjectId>,
    ) -> Result<(), String> {
        let mut themes = self.themes.lock().unwrap();
        for theme in themes.iter_mut().filter(|theme| &theme.tenant_id == tenant_id) {
            if except_theme_id.is_some_and(|id| &theme.id == id) {
                continue;
            }
            theme.is_default = false;
        }
        Ok(())
    }

    async fn delete_theme(&self, tenant_id: &ObjectId, theme_id: &ObjectId) -> Result<bool, String> {
        let mut themes = self.themes.lock().unwrap();
        let before = themes.len();
        themes.retain(|theme| !(&theme.tenant_id == tenant_id && &theme.id == theme_id));
        Ok(themes.len() < before)
    }
}
