//! Thin orchestrator for domain lifecycle + quota enforcement.
//!
//! Exists so `DomainsRepository::create_domain` is never called directly
//! from a transport layer — the quota check must travel with the create,
//! same rule as links/conversions/users (see CLAUDE.md "Quota enforcement").

use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::models::{Domain, DomainError, DomainRole};
use super::repo::DomainsRepository;
use crate::services::billing::quota::{QuotaChecker, Resource};

crate::impl_container!(DomainsService);
pub struct DomainsService {
    repo: Arc<dyn DomainsRepository>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl DomainsService {
    pub fn new(repo: Arc<dyn DomainsRepository>, quota: Option<Arc<dyn QuotaChecker>>) -> Self {
        Self { repo, quota }
    }

    /// Create a verified-on-demand domain record. Enforces both the alt-domain
    /// cap (hardcoded, 1 per tenant) and the tier domain quota (via
    /// `QuotaService::check`) before calling the repo.
    pub async fn create_domain(
        &self,
        tenant_id: ObjectId,
        domain: String,
        verification_token: String,
        role: DomainRole,
    ) -> Result<Domain, DomainError> {
        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::CreateDomain).await?;
        }

        if role == DomainRole::Alternate {
            if let Ok(Some(_)) = self.repo.find_alternate_by_tenant(&tenant_id).await {
                return Err(DomainError::AlternateLimit);
            }
        }

        if self
            .repo
            .find_by_domain(&domain)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            return Err(DomainError::AlreadyRegistered);
        }

        match self
            .repo
            .create_domain(tenant_id, domain, verification_token, role)
            .await
        {
            Ok(d) => Ok(d),
            Err(e) if e.contains("E11000") => Err(DomainError::AlreadyRegistered),
            Err(e) => Err(DomainError::Internal(e)),
        }
    }
}
