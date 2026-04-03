use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::{Domain, DomainRole};

// ── Trait ──

#[async_trait]
pub trait DomainsRepository: Send + Sync {
    async fn create_domain(
        &self,
        tenant_id: ObjectId,
        domain: String,
        verification_token: String,
        role: DomainRole,
    ) -> Result<Domain, String>;

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Domain>, String>;

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Domain>, String>;

    async fn delete_domain(&self, tenant_id: &ObjectId, domain: &str) -> Result<bool, String>;

    async fn mark_verified(&self, domain: &str) -> Result<(), String>;

    async fn find_alternate_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<Domain>, String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct DomainsRepo {
    domains: Collection<Domain>,
}

impl DomainsRepo {
    pub async fn new(database: &Database) -> Self {
        let domains = database.collection::<Domain>("domains");

        ensure_index!(
            domains,
            doc! { "domain": 1 },
            IndexOptions::builder().unique(true).build(),
            "domain_unique"
        );
        ensure_index!(domains, doc! { "tenant_id": 1 }, "domains_tenant");

        DomainsRepo { domains }
    }
}

#[async_trait]
impl DomainsRepository for DomainsRepo {
    async fn create_domain(
        &self,
        tenant_id: ObjectId,
        domain: String,
        verification_token: String,
        role: DomainRole,
    ) -> Result<Domain, String> {
        let doc = Domain {
            id: ObjectId::new(),
            tenant_id,
            domain,
            verified: false,
            verification_token,
            role,
            created_at: DateTime::now(),
        };
        self.domains
            .insert_one(&doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(doc)
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Domain>, String> {
        self.domains
            .find_one(doc! { "domain": domain })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Domain>, String> {
        let mut cursor = self
            .domains
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut domains = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            domains.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(domains)
    }

    async fn delete_domain(&self, tenant_id: &ObjectId, domain: &str) -> Result<bool, String> {
        let result = self
            .domains
            .delete_one(doc! { "tenant_id": tenant_id, "domain": domain })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }

    async fn mark_verified(&self, domain: &str) -> Result<(), String> {
        self.domains
            .update_one(
                doc! { "domain": domain },
                doc! { "$set": { "verified": true } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_alternate_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<Domain>, String> {
        self.domains
            .find_one(doc! { "tenant_id": tenant_id, "role": "alternate", "verified": true })
            .await
            .map_err(|e| e.to_string())
    }
}
