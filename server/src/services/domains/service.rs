//! Thin orchestrator for domain lifecycle + quota enforcement.
//!
//! Exists so `DomainsRepository::create_domain` is never called directly
//! from a transport layer — the quota check must travel with the create,
//! same rule as links/conversions/users (see CLAUDE.md "Quota enforcement").

use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::models::{Domain, DomainRole};
use super::repo::DomainsRepository;
use crate::services::billing::quota::{QuotaChecker, QuotaError, Resource};

#[derive(Debug)]
pub enum DomainError {
    AlreadyRegistered,
    AlternateLimit,
    QuotaExceeded(QuotaError),
    Internal(String),
}

impl From<QuotaError> for DomainError {
    fn from(err: QuotaError) -> Self {
        DomainError::QuotaExceeded(err)
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRegistered => write!(f, "Domain already registered"),
            Self::AlternateLimit => write!(f, "Only one alternate domain allowed per team"),
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

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
