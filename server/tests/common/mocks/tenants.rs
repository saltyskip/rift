use async_trait::async_trait;
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
}
