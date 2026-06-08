use async_trait::async_trait;
use std::sync::Mutex;

use rift::core::public_id::{TenantId, UserId};
use rift::services::auth::users::models::UserDoc;
use rift::services::auth::users::repo::UsersRepository;

#[derive(Default)]
pub struct MockUsersRepo {
    pub users: Mutex<Vec<UserDoc>>,
}

#[async_trait]
impl UsersRepository for MockUsersRepo {
    async fn create(&self, doc: &UserDoc) -> Result<(), String> {
        self.users.lock().unwrap().push(doc.clone());
        Ok(())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDoc>, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn find_by_tenant_and_email(
        &self,
        tenant_id: &TenantId,
        email: &str,
    ) -> Result<Option<UserDoc>, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.tenant_id == *tenant_id && u.email == email)
            .cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<UserDoc>, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.tenant_id == *tenant_id)
            .cloned()
            .collect())
    }

    async fn count_verified_by_tenant(&self, tenant_id: &TenantId) -> Result<i64, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.tenant_id == *tenant_id && u.verified)
            .count() as i64)
    }

    async fn delete(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<bool, String> {
        let mut users = self.users.lock().unwrap();
        let len = users.len();
        users.retain(|u| !(u.id.as_ref() == Some(user_id) && u.tenant_id == *tenant_id));
        Ok(users.len() < len)
    }

    async fn mark_verified(&self, email: &str) -> Result<Option<UserDoc>, String> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.email == email) {
            user.verified = true;
            Ok(Some(user.clone()))
        } else {
            Ok(None)
        }
    }

    async fn set_invite_expiry(
        &self,
        tenant_id: &TenantId,
        email: &str,
        expires_at: mongodb::bson::DateTime,
    ) -> Result<bool, String> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users
            .iter_mut()
            .find(|u| u.tenant_id == *tenant_id && u.email == email)
        {
            user.invite_expires_at = Some(expires_at);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
