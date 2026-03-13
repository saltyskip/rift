use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};

use super::models::Domain;

macro_rules! ensure_index {
    ($col:expr, $keys:expr, $opts:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(IndexModel::builder().keys($keys).options($opts).build())
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
    ($col:expr, $keys:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(IndexModel::builder().keys($keys).build())
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
}

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

    pub async fn create_domain(
        &self,
        tenant_id: ObjectId,
        domain: String,
        verification_token: String,
    ) -> Result<Domain, mongodb::error::Error> {
        let doc = Domain {
            id: ObjectId::new(),
            tenant_id,
            domain,
            verified: false,
            verification_token,
            created_at: DateTime::now(),
        };
        self.domains.insert_one(&doc).await?;
        Ok(doc)
    }

    pub async fn find_by_domain(&self, domain: &str) -> Result<Option<Domain>, mongodb::error::Error> {
        self.domains.find_one(doc! { "domain": domain }).await
    }

    pub async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Domain>, mongodb::error::Error> {
        let mut cursor = self
            .domains
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await?;

        let mut domains = Vec::new();
        while cursor.advance().await? {
            domains.push(cursor.deserialize_current()?);
        }
        Ok(domains)
    }

    pub async fn delete_domain(
        &self,
        tenant_id: &ObjectId,
        domain: &str,
    ) -> Result<bool, mongodb::error::Error> {
        let result = self
            .domains
            .delete_one(doc! { "tenant_id": tenant_id, "domain": domain })
            .await?;
        Ok(result.deleted_count > 0)
    }

    pub async fn mark_verified(&self, domain: &str) -> Result<(), mongodb::error::Error> {
        self.domains
            .update_one(
                doc! { "domain": domain },
                doc! { "$set": { "verified": true } },
            )
            .await?;
        Ok(())
    }
}
