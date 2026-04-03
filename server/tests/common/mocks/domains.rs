use async_trait::async_trait;
use mongodb::bson::{oid::ObjectId, DateTime};
use std::sync::Mutex;

use rift::services::domains::models::{Domain, DomainRole};
use rift::services::domains::repo::DomainsRepository;

#[derive(Default)]
pub struct MockDomainsRepo {
    pub domains: Mutex<Vec<Domain>>,
}

#[async_trait]
impl DomainsRepository for MockDomainsRepo {
    async fn create_domain(
        &self,
        tenant_id: ObjectId,
        domain: String,
        verification_token: String,
        role: DomainRole,
    ) -> Result<Domain, String> {
        let mut domains = self.domains.lock().unwrap();
        if domains.iter().any(|d| d.domain == domain) {
            return Err("E11000 duplicate key".to_string());
        }
        let doc = Domain {
            id: ObjectId::new(),
            tenant_id,
            domain,
            verified: false,
            verification_token,
            role,
            created_at: DateTime::now(),
        };
        domains.push(doc.clone());
        Ok(doc)
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Domain>, String> {
        Ok(self
            .domains
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.domain == domain)
            .cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Domain>, String> {
        Ok(self
            .domains
            .lock()
            .unwrap()
            .iter()
            .filter(|d| &d.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn delete_domain(&self, tenant_id: &ObjectId, domain: &str) -> Result<bool, String> {
        let mut domains = self.domains.lock().unwrap();
        let len_before = domains.len();
        domains.retain(|d| !(&d.tenant_id == tenant_id && d.domain == domain));
        Ok(domains.len() < len_before)
    }

    async fn mark_verified(&self, domain: &str) -> Result<(), String> {
        let mut domains = self.domains.lock().unwrap();
        if let Some(d) = domains.iter_mut().find(|d| d.domain == domain) {
            d.verified = true;
        }
        Ok(())
    }

    async fn find_alternate_by_tenant(
        &self,
        tenant_id: &ObjectId,
    ) -> Result<Option<Domain>, String> {
        Ok(self
            .domains
            .lock()
            .unwrap()
            .iter()
            .find(|d| &d.tenant_id == tenant_id && d.role == DomainRole::Alternate && d.verified)
            .cloned())
    }
}
