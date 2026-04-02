use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::auth::tenants::repo::{TenantDoc, TenantsRepository};

#[derive(Default)]
pub struct MockTenantsRepo {
    pub tenants: Mutex<Vec<TenantDoc>>,
}

#[async_trait]
impl TenantsRepository for MockTenantsRepo {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String> {
        self.tenants.lock().unwrap().push(doc.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<TenantDoc>, String> {
        Ok(self
            .tenants
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id.as_ref() == Some(id))
            .cloned())
    }
}
