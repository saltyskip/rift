use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::repo::{TenantDoc, TenantsRepository};

/// Tenant lifecycle primitives shared across signup (and later billing / agent)
/// flows. Keeps credential-specific concerns (email owners, wallet credentials,
/// secret keys) out of the tenant layer.
pub struct TenantsService {
    tenants_repo: Arc<dyn TenantsRepository>,
}

impl TenantsService {
    pub fn new(tenants_repo: Arc<dyn TenantsRepository>) -> Self {
        Self { tenants_repo }
    }

    /// Create a bare tenant with default limits and return its id. Callers are
    /// responsible for attaching an owner (email user, wallet credential, etc.)
    /// immediately after.
    pub async fn create_blank(&self) -> Result<ObjectId, String> {
        let id = ObjectId::new();
        let doc = TenantDoc {
            id: Some(id),
            monthly_quota: 100,
            created_at: mongodb::bson::DateTime::now(),
        };
        self.tenants_repo.create(&doc).await?;
        Ok(id)
    }
}
