use chrono::{Datelike, Utc};
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::limits::{limits_for, PlanLimits};
use super::models::BillingError;
use super::repos::event_counters::EventCountersRepository;
use super::service::BillingService;

/// Quotable resource categories. Each maps to a specific enforcement path.
/// `TrackEvent` covers both click and conversion writes — they share the
/// `max_events_per_month` limit on the pricing page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    CreateLink,
    TrackEvent,
    CreateDomain,
    InviteTeamMember,
    CreateWebhook,
}

impl Resource {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CreateLink => "create_link",
            Self::TrackEvent => "track_event",
            Self::CreateDomain => "create_domain",
            Self::InviteTeamMember => "invite_team_member",
            Self::CreateWebhook => "create_webhook",
        }
    }
}

/// Outcome of a quota check. In Phase A-1 (log-only) we log `Exceeded` and
/// continue; Phase A-2 will return it as a `402 Payment Required` to clients.
#[derive(Debug)]
pub enum QuotaError {
    Exceeded {
        resource: Resource,
        limit: u64,
        current: u64,
    },
    Billing(BillingError),
}

impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exceeded {
                resource,
                limit,
                current,
            } => write!(
                f,
                "quota exceeded on {} ({}/{})",
                resource.code(),
                current,
                limit
            ),
            Self::Billing(e) => write!(f, "billing error: {e}"),
        }
    }
}

impl From<BillingError> for QuotaError {
    fn from(err: BillingError) -> Self {
        QuotaError::Billing(err)
    }
}

/// Per-resource counter for "things already owned by this tenant" — used by
/// `QuotaService::check` to decide whether a new create is allowed. Each
/// existing repo already has (or gets) a `count_by_tenant` for exactly this.
#[async_trait::async_trait]
pub trait ResourceCounts: Send + Sync {
    async fn count(&self, tenant_id: &ObjectId, resource: Resource) -> Result<u64, String>;
}

/// The quota gatekeeper. All enforcement points in the service layer call
/// this. MCP tools and future transports call the same entry points —
/// there's no HTTP middleware equivalent on purpose, so new transports can't
/// accidentally bypass enforcement.
///
/// Phase A-1 runs in log-only mode: the service always returns `Ok(())` but
/// logs would-be rejections via `tracing::warn!`. Phase A-2 will introduce
/// an `EnforcementMode` and flip to hard rejection.
pub struct QuotaService {
    billing: Arc<BillingService>,
    counters: Arc<dyn EventCountersRepository>,
    resource_counts: Arc<dyn ResourceCounts>,
}

impl QuotaService {
    pub fn new(
        billing: Arc<BillingService>,
        counters: Arc<dyn EventCountersRepository>,
        resource_counts: Arc<dyn ResourceCounts>,
    ) -> Self {
        Self {
            billing,
            counters,
            resource_counts,
        }
    }

    /// Observe (and in the case of `TrackEvent`, atomically record) a quota
    /// check. In log-only mode the return is always `Ok(())` — the `Err`
    /// variant exists so the call site is future-ready for enforcement.
    pub async fn check(&self, tenant_id: &ObjectId, resource: Resource) -> Result<(), QuotaError> {
        let tier = self.billing.effective_tier(tenant_id).await?;
        let limits = limits_for(tier);
        let max = match limit_for_resource(&limits, resource) {
            Some(m) => m,
            None => return Ok(()), // unlimited
        };

        let exceeded = match resource {
            Resource::TrackEvent => {
                let period = current_period();
                let within = self
                    .counters
                    .increment_if_below(tenant_id, &period, Some(max))
                    .await
                    .map_err(|e| QuotaError::Billing(BillingError::Internal(e)))?;
                if within {
                    None
                } else {
                    Some((max, max))
                }
            }
            _ => {
                let current = self
                    .resource_counts
                    .count(tenant_id, resource)
                    .await
                    .map_err(|e| QuotaError::Billing(BillingError::Internal(e)))?;
                if current < max {
                    None
                } else {
                    Some((max, current))
                }
            }
        };

        if let Some((limit, current)) = exceeded {
            let err = QuotaError::Exceeded {
                resource,
                limit,
                current,
            };
            tracing::warn!(quota_error = %err, "quota_check_log_only_would_reject");
        }
        Ok(())
    }
}

fn limit_for_resource(limits: &PlanLimits, resource: Resource) -> Option<u64> {
    match resource {
        Resource::CreateLink => limits.max_links,
        Resource::TrackEvent => limits.max_events_per_month,
        Resource::CreateDomain => limits.max_domains,
        Resource::InviteTeamMember => limits.max_team_members,
        Resource::CreateWebhook => limits.max_webhooks,
    }
}

fn current_period() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::auth::tenants::repo::{PlanTier, TenantDoc, TenantsRepository};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockTenants {
        tenants: Mutex<Vec<TenantDoc>>,
    }

    #[async_trait]
    impl TenantsRepository for MockTenants {
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
        async fn find_by_stripe_customer_id(
            &self,
            _customer_id: &str,
        ) -> Result<Option<TenantDoc>, String> {
            Ok(None)
        }
        async fn apply_subscription_update(
            &self,
            _tenant_id: &ObjectId,
            _update: crate::services::auth::tenants::repo::SubscriptionUpdate,
        ) -> Result<bool, String> {
            Ok(true)
        }
        async fn clear_subscription(&self, _tenant_id: &ObjectId) -> Result<bool, String> {
            Ok(true)
        }
    }

    #[derive(Default)]
    struct MockCounts {
        counts: Mutex<std::collections::HashMap<&'static str, u64>>,
    }

    impl MockCounts {
        fn set(&self, resource: Resource, n: u64) {
            self.counts.lock().unwrap().insert(resource.code(), n);
        }
    }

    #[async_trait]
    impl ResourceCounts for MockCounts {
        async fn count(&self, _tenant_id: &ObjectId, resource: Resource) -> Result<u64, String> {
            Ok(*self
                .counts
                .lock()
                .unwrap()
                .get(resource.code())
                .unwrap_or(&0))
        }
    }

    #[derive(Default)]
    struct MockCounters {
        count: Mutex<u64>,
    }

    #[async_trait]
    impl EventCountersRepository for MockCounters {
        async fn increment_if_below(
            &self,
            _tenant_id: &ObjectId,
            _period: &str,
            max: Option<u64>,
        ) -> Result<bool, String> {
            let mut c = self.count.lock().unwrap();
            match max {
                None => {
                    *c += 1;
                    Ok(true)
                }
                Some(m) if *c < m => {
                    *c += 1;
                    Ok(true)
                }
                Some(_) => Ok(false),
            }
        }
    }

    async fn setup_with_plan(
        plan: PlanTier,
    ) -> (QuotaService, ObjectId, Arc<MockCounts>, Arc<MockCounters>) {
        let tenants = Arc::new(MockTenants::default());
        let id = ObjectId::new();
        tenants
            .create(&TenantDoc {
                id: Some(id),
                plan_tier: plan,
                ..TenantDoc::default()
            })
            .await
            .unwrap();
        let counts = Arc::new(MockCounts::default());
        let counters = Arc::new(MockCounters::default());
        let billing = Arc::new(BillingService::new(
            tenants.clone() as Arc<dyn TenantsRepository>
        ));
        let q = QuotaService::new(
            billing,
            counters.clone() as Arc<dyn EventCountersRepository>,
            counts.clone() as Arc<dyn ResourceCounts>,
        );
        (q, id, counts, counters)
    }

    #[tokio::test]
    async fn under_limit_returns_ok() {
        let (q, id, counts, _) = setup_with_plan(PlanTier::Free).await;
        counts.set(Resource::CreateLink, 49);
        q.check(&id, Resource::CreateLink).await.unwrap();
    }

    #[tokio::test]
    async fn at_limit_logs_but_passes_in_log_only() {
        let (q, id, counts, _) = setup_with_plan(PlanTier::Free).await;
        counts.set(Resource::CreateLink, 50); // Free max
        q.check(&id, Resource::CreateLink).await.unwrap();
    }

    #[tokio::test]
    async fn unlimited_team_on_business() {
        let (q, id, counts, _) = setup_with_plan(PlanTier::Business).await;
        counts.set(Resource::InviteTeamMember, 10_000);
        q.check(&id, Resource::InviteTeamMember).await.unwrap();
    }

    #[tokio::test]
    async fn track_event_uses_atomic_counter() {
        let (q, id, _, counters) = setup_with_plan(PlanTier::Free).await;
        // Free max_events_per_month = 10_000
        *counters.count.lock().unwrap() = 10_000;
        // Over-limit — logs-and-passes in log-only mode.
        q.check(&id, Resource::TrackEvent).await.unwrap();
    }

    #[tokio::test]
    async fn unknown_tenant_propagates_billing_error() {
        let (q, _, _, _) = setup_with_plan(PlanTier::Free).await;
        let err = q.check(&ObjectId::new(), Resource::CreateLink).await;
        assert!(matches!(err, Err(QuotaError::Billing(_))));
    }

    #[test]
    fn quota_error_display_includes_fields() {
        let err = QuotaError::Exceeded {
            resource: Resource::CreateLink,
            limit: 50,
            current: 50,
        };
        let rendered = err.to_string();
        assert!(rendered.contains("create_link"));
        assert!(rendered.contains("50"));
    }
}
