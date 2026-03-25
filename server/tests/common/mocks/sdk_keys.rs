use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::api::sdk_keys::models::SdkKeyDoc;
use rift::api::sdk_keys::repo::SdkKeysRepository;

#[derive(Default)]
pub struct MockSdkKeysRepo {
    pub keys: Mutex<Vec<SdkKeyDoc>>,
}

#[async_trait]
impl SdkKeysRepository for MockSdkKeysRepo {
    async fn create_key(&self, doc: &SdkKeyDoc) -> Result<(), String> {
        self.keys.lock().unwrap().push(doc.clone());
        Ok(())
    }

    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SdkKeyDoc>, String> {
        Ok(self
            .keys
            .lock()
            .unwrap()
            .iter()
            .find(|k| k.key_hash == key_hash && !k.revoked)
            .cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SdkKeyDoc>, String> {
        Ok(self
            .keys
            .lock()
            .unwrap()
            .iter()
            .filter(|k| &k.tenant_id == tenant_id && !k.revoked)
            .cloned()
            .collect())
    }

    async fn revoke(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String> {
        let mut keys = self.keys.lock().unwrap();
        if let Some(key) = keys
            .iter_mut()
            .find(|k| &k.id == key_id && &k.tenant_id == tenant_id)
        {
            key.revoked = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
