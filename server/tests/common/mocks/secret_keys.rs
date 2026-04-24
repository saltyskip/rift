use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::auth::secret_keys::repo::{SecretKeyDoc, SecretKeysRepository};

#[derive(Default)]
pub struct MockSecretKeysRepo {
    pub keys: Mutex<Vec<SecretKeyDoc>>,
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
}
