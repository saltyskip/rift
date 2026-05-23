use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::app_users::models::{AppUserDoc, AppUserUpsert};
use rift::services::app_users::repo::AppUsersRepository;

/// In-memory app_users mock. Same multi-install accumulation semantics as
/// the real repo (`$addToSet` on `install_ids`).
#[derive(Default)]
pub struct MockAppUsersRepo {
    rows: Mutex<Vec<AppUserDoc>>,
}

#[async_trait]
impl AppUsersRepository for MockAppUsersRepo {
    async fn upsert_with_install(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
        install_id: &str,
    ) -> Result<AppUserUpsert, String> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(row) = rows
            .iter_mut()
            .find(|r| &r.tenant_id == tenant_id && r.user_id == user_id)
        {
            if row.install_ids.iter().any(|i| i == install_id) {
                Ok(AppUserUpsert::AlreadyPresent)
            } else {
                row.install_ids.push(install_id.to_string());
                Ok(AppUserUpsert::InstallAdded)
            }
        } else {
            rows.push(AppUserDoc {
                id: Some(ObjectId::new()),
                tenant_id: *tenant_id,
                user_id: user_id.to_string(),
                install_ids: vec![install_id.to_string()],
                identified_at: mongodb::bson::DateTime::now(),
                last_seen_at: mongodb::bson::DateTime::now(),
                last_app_version: None,
                last_platform: None,
                last_os_version: None,
                last_device_model: None,
                last_device_manufacturer: None,
                last_locale: None,
                last_region: None,
                last_timezone: None,
            });
            Ok(AppUserUpsert::Created)
        }
    }

    async fn find_by_user_id(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<AppUserDoc>, String> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .find(|r| &r.tenant_id == tenant_id && r.user_id == user_id)
            .cloned())
    }

    async fn find_user_id_for_install(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
    ) -> Result<Option<String>, String> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .find(|r| &r.tenant_id == tenant_id && r.install_ids.iter().any(|i| i == install_id))
            .map(|r| r.user_id.clone()))
    }
}
