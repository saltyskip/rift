use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::auth::users::repo::{UserDoc, UsersRepository};

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
        tenant_id: &ObjectId,
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

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<UserDoc>, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.tenant_id == *tenant_id)
            .cloned()
            .collect())
    }

    async fn count_verified_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.tenant_id == *tenant_id && u.verified)
            .count() as i64)
    }

    async fn delete(&self, tenant_id: &ObjectId, user_id: &ObjectId) -> Result<bool, String> {
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

    async fn upsert_by_email(&self, doc: &UserDoc) -> Result<(), String> {
        let mut users = self.users.lock().unwrap();
        if let Some(existing) = users.iter_mut().find(|u| u.email == doc.email) {
            existing.tenant_id = doc.tenant_id;
            existing.is_owner = doc.is_owner;
        } else {
            users.push(doc.clone());
        }
        Ok(())
    }
}
