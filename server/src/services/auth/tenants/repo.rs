use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

// ── Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub monthly_quota: i64,
    pub created_at: bson::DateTime,
}

// ── Trait ──

#[async_trait]
pub trait TenantsRepository: Send + Sync {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct TenantsRepo {
    tenants: Collection<TenantDoc>,
}

impl TenantsRepo {
    pub async fn new(database: &Database) -> Self {
        let tenants = database.collection::<TenantDoc>("tenants");
        TenantsRepo { tenants }
    }
}

#[async_trait]
impl TenantsRepository for TenantsRepo {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String> {
        self.tenants
            .insert_one(doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
