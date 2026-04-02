use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::auth::secret_keys::repo::{
    SecretKeyCreateRequestDoc, SecretKeyDoc, SecretKeysRepository,
};

#[derive(Default)]
pub struct MockSecretKeysRepo {
    pub keys: Mutex<Vec<SecretKeyDoc>>,
    pub requests: Mutex<Vec<SecretKeyCreateRequestDoc>>,
}

#[async_trait]
impl SecretKeysRepository for MockSecretKeysRepo {
    async fn create_key(&self, doc: &SecretKeyDoc) -> Result<(), String> {
        self.keys.lock().unwrap().push(doc.clone());
        Ok(())
    }

    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SecretKeyDoc>, String> {
        Ok(self
            .keys
            .lock()
            .unwrap()
            .iter()
            .find(|k| k.key_hash == key_hash)
            .cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SecretKeyDoc>, String> {
        Ok(self
            .keys
            .lock()
            .unwrap()
            .iter()
            .filter(|k| k.tenant_id == *tenant_id)
            .cloned()
            .collect())
    }

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String> {
        Ok(self
            .keys
            .lock()
            .unwrap()
            .iter()
            .filter(|k| k.tenant_id == *tenant_id)
            .count() as i64)
    }

    async fn delete_key(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String> {
        let mut keys = self.keys.lock().unwrap();
        let len = keys.len();
        keys.retain(|k| !(k.id == *key_id && k.tenant_id == *tenant_id));
        Ok(keys.len() < len)
    }

    async fn find_pending_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<Option<SecretKeyCreateRequestDoc>, String> {
        let now = mongodb::bson::DateTime::now();
        Ok(self
            .requests
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.tenant_id == *tenant_id && r.user_id == *user_id && r.expires_at > now)
            .cloned())
    }

    async fn create_request(&self, doc: &SecretKeyCreateRequestDoc) -> Result<(), String> {
        let mut requests = self.requests.lock().unwrap();
        requests.retain(|r| !(r.tenant_id == doc.tenant_id && r.user_id == doc.user_id));
        requests.push(doc.clone());
        Ok(())
    }

    async fn validate_and_consume_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
        token_hash: &str,
    ) -> Result<bool, String> {
        let now = mongodb::bson::DateTime::now();
        let mut requests = self.requests.lock().unwrap();
        let len = requests.len();
        requests.retain(|r| {
            !(r.tenant_id == *tenant_id
                && r.user_id == *user_id
                && r.token_hash == token_hash
                && r.expires_at > now
                && r.attempts < 5)
        });
        Ok(requests.len() < len)
    }

    async fn increment_request_attempts(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<i32, String> {
        let now = mongodb::bson::DateTime::now();
        let mut requests = self.requests.lock().unwrap();
        if let Some(req) = requests
            .iter_mut()
            .find(|r| r.tenant_id == *tenant_id && r.user_id == *user_id && r.expires_at > now)
        {
            req.attempts += 1;
            Ok(req.attempts)
        } else {
            Ok(0)
        }
    }
}
