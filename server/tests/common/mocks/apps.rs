use async_trait::async_trait;
use std::sync::Mutex;

use rift::core::public_id::{AppId, TenantId};
use rift::services::apps::models::App;
use rift::services::apps::repo::AppsRepository;

#[derive(Default)]
pub struct MockAppsRepo {
    pub apps: Mutex<Vec<App>>,
}

#[async_trait]
impl AppsRepository for MockAppsRepo {
    async fn create_or_update(&self, app: App) -> Result<App, String> {
        let mut apps = self.apps.lock().unwrap();
        if let Some(existing) = apps
            .iter_mut()
            .find(|a| a.tenant_id == app.tenant_id && a.platform == app.platform)
        {
            existing.bundle_id = app.bundle_id.clone();
            existing.team_id = app.team_id.clone();
            existing.package_name = app.package_name.clone();
            existing.sha256_fingerprints = app.sha256_fingerprints.clone();
            existing.app_name = app.app_name.clone();
            existing.icon_url = app.icon_url.clone();
            existing.theme_color = app.theme_color.clone();
            return Ok(app);
        }
        apps.push(app.clone());
        Ok(app)
    }

    async fn list_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<App>, String> {
        Ok(self
            .apps
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.tenant_id == *tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_platform(
        &self,
        tenant_id: &TenantId,
        platform: &str,
    ) -> Result<Option<App>, String> {
        Ok(self
            .apps
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.tenant_id == *tenant_id && a.platform == platform)
            .cloned())
    }

    async fn delete_app(&self, tenant_id: &TenantId, app_id: &AppId) -> Result<bool, String> {
        let mut apps = self.apps.lock().unwrap();
        let len_before = apps.len();
        apps.retain(|a| !(a.tenant_id == *tenant_id && a.id == *app_id));
        Ok(apps.len() < len_before)
    }
}
