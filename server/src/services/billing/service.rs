use async_trait::async_trait;
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::effective_tier::effective_tier;
use super::limits::limits_for;
use super::models::{BillingError, BillingStatus};
use crate::services::auth::tenants::repo::{PlanTier, TenantsRepository};

/// The read-side of billing state that most services actually need.
///
/// Quota checks ask for the tenant's effective tier; event insert paths ask
/// for the retention bucket. Neither needs the full BillingService — which
/// also owns subscription lifecycle methods (`status`, Stripe update apply,
/// etc.) that most consumers don't care about.
///
/// Injecting this trait instead of `Arc<BillingService>` keeps the type
/// surface of downstream services small and makes them testable with
/// fake tier data.
#[async_trait]
pub trait TierResolver: Send + Sync {
    async fn effective_tier(&self, tenant_id: &ObjectId) -> Result<PlanTier, BillingError>;
    async fn retention_bucket_for_tenant(&self, tenant_id: &ObjectId) -> &'static str;
}

crate::impl_container!(BillingService);
/// Central entry point for anything that cares about a tenant's billing state.
/// Every status endpoint, quota check, webhook handler, and admin operation
/// routes through here so "what plan is this tenant on" has exactly one
/// answer.
pub struct BillingService {
    tenants_repo: Arc<dyn TenantsRepository>,
}

impl BillingService {
    pub fn new(tenants_repo: Arc<dyn TenantsRepository>) -> Self {
        Self { tenants_repo }
    }

    pub async fn status(&self, tenant_id: &ObjectId) -> Result<BillingStatus, BillingError> {
        let tenant = self
            .tenants_repo
            .find_by_id(tenant_id)
            .await
            .map_err(BillingError::Internal)?
            .ok_or(BillingError::TenantNotFound)?;
        let now = bson::DateTime::now();
        let eff = effective_tier(&tenant, now);
        let comp_active =
            tenant.comp_tier.is_some() && tenant.comp_until.map(|u| now < u).unwrap_or(true);
        Ok(BillingStatus {
            plan_tier: tenant.plan_tier,
            effective_tier: eff,
            comp_active,
            billing_method: tenant.billing_method,
            status: tenant.status,
            current_period_end: tenant.current_period_end,
        })
    }
}

// Production impl. Downstream services inject `Arc<dyn TierResolver>` to
// stay decoupled from BillingService's subscription-lifecycle surface.
#[async_trait]
impl TierResolver for BillingService {
    async fn effective_tier(&self, tenant_id: &ObjectId) -> Result<PlanTier, BillingError> {
        let tenant = self
            .tenants_repo
            .find_by_id(tenant_id)
            .await
            .map_err(BillingError::Internal)?
            .ok_or(BillingError::TenantNotFound)?;
        Ok(effective_tier(&tenant, bson::DateTime::now()))
    }

    async fn retention_bucket_for_tenant(&self, tenant_id: &ObjectId) -> &'static str {
        match self.effective_tier(tenant_id).await {
            Ok(tier) => limits_for(tier).retention_bucket,
            Err(_) => "30d",
        }
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
