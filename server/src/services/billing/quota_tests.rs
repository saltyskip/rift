use super::*;
use crate::services::auth::tenants::repo::{PlanTier, TenantDoc, TenantsRepository};
use crate::services::billing::service::BillingService;
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
    async fn find_by_owner_email(&self, _email: &str) -> Result<Option<TenantDoc>, String> {
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

async fn setup_with_plan_mode(
    plan: PlanTier,
    mode: EnforcementMode,
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
    )) as Arc<dyn TierResolver>;
    let q = QuotaService::new(
        billing,
        counters.clone() as Arc<dyn EventCountersRepository>,
        counts.clone() as Arc<dyn ResourceCounts>,
        mode,
    );
    (q, id, counts, counters)
}

async fn setup_with_plan(
    plan: PlanTier,
) -> (QuotaService, ObjectId, Arc<MockCounts>, Arc<MockCounters>) {
    setup_with_plan_mode(plan, EnforcementMode::LogOnly).await
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
async fn at_limit_rejects_in_enforce_mode() {
    let (q, id, counts, _) = setup_with_plan_mode(PlanTier::Free, EnforcementMode::Enforce).await;
    counts.set(Resource::CreateLink, 50);
    let err = q.check(&id, Resource::CreateLink).await.unwrap_err();
    match err {
        QuotaError::Exceeded {
            resource,
            limit,
            current,
        } => {
            assert_eq!(resource, Resource::CreateLink);
            assert_eq!(limit, 50);
            assert_eq!(current, 50);
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn enforcement_mode_env_parses() {
    assert_eq!(
        EnforcementMode::from_env_str("enforce"),
        EnforcementMode::Enforce
    );
    assert_eq!(
        EnforcementMode::from_env_str("ENFORCE"),
        EnforcementMode::Enforce
    );
    assert_eq!(
        EnforcementMode::from_env_str("log_only"),
        EnforcementMode::LogOnly
    );
    assert_eq!(EnforcementMode::from_env_str(""), EnforcementMode::LogOnly);
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

#[tokio::test]
async fn noop_checker_always_ok() {
    let c = NoopQuotaChecker;
    c.check(&ObjectId::new(), Resource::CreateLink)
        .await
        .unwrap();
}

#[tokio::test]
async fn deny_checker_always_errs() {
    let c = DenyQuotaChecker { limit: 42 };
    let err = c
        .check(&ObjectId::new(), Resource::CreateLink)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        QuotaError::Exceeded {
            resource: Resource::CreateLink,
            limit: 42,
            ..
        }
    ));
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
