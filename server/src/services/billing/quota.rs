use async_trait::async_trait;
use chrono::{Datelike, Utc};
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::limits::{limits_for, PlanLimits};
use super::models::BillingError;
use super::repos::event_counters::EventCountersRepository;
use super::service::TierResolver;

/// The quota decision surface used by every service that creates a resource
/// or tracks an event.
///
/// Services inject `Arc<dyn QuotaChecker>` rather than the concrete
/// `QuotaService`, which keeps the internal fanout (billing, event
/// counters, per-domain resource counts) invisible at the call site.
///
/// Production wiring uses `QuotaService`; tests use `NoopQuotaChecker`
/// (always Ok) or `DenyQuotaChecker` (configurable rejection) — see
/// `#[cfg(test)]` at the bottom of this file.
#[async_trait]
pub trait QuotaChecker: Send + Sync {
    async fn check(&self, tenant_id: &ObjectId, resource: Resource) -> Result<(), QuotaError>;
}

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

/// Whether quota checks hard-reject or just log the would-be rejection.
///
/// `LogOnly` is the safe default — every code path calls `QuotaService::check`
/// but it always returns `Ok(())`, emitting `tracing::warn!` when a tenant
/// would have been rejected. `Enforce` flips `QuotaError::Exceeded` into a
/// real error the caller maps to `402 Payment Required`.
///
/// Controlled by `QUOTA_ENFORCEMENT=enforce` (default: log_only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    LogOnly,
    Enforce,
}

impl EnforcementMode {
    pub fn from_env_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("enforce") {
            Self::Enforce
        } else {
            Self::LogOnly
        }
    }
}

/// The concrete production quota gatekeeper. Injected behind
/// `Arc<dyn QuotaChecker>` so services don't see the internal fanout.
pub struct QuotaService {
    billing: Arc<dyn TierResolver>,
    counters: Arc<dyn EventCountersRepository>,
    resource_counts: Arc<dyn ResourceCounts>,
    mode: EnforcementMode,
}

impl QuotaService {
    pub fn new(
        billing: Arc<dyn TierResolver>,
        counters: Arc<dyn EventCountersRepository>,
        resource_counts: Arc<dyn ResourceCounts>,
        mode: EnforcementMode,
    ) -> Self {
        Self {
            billing,
            counters,
            resource_counts,
            mode,
        }
    }
}

#[async_trait]
impl QuotaChecker for QuotaService {
    /// Observe (and for `TrackEvent`, atomically record) a quota check.
    ///
    /// - `LogOnly`: returns `Ok(())` always, logs would-be rejections.
    /// - `Enforce`: returns `Err(QuotaError::Exceeded { ... })` when over
    ///   limit. Caller renders as `402 Payment Required`.
    async fn check(&self, tenant_id: &ObjectId, resource: Resource) -> Result<(), QuotaError> {
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
            match self.mode {
                EnforcementMode::LogOnly => {
                    tracing::warn!(
                        quota_error = %err,
                        mode = "log_only",
                        "quota_check_would_reject"
                    );
                    return Ok(());
                }
                EnforcementMode::Enforce => {
                    tracing::info!(quota_error = %err, mode = "enforce", "quota_rejected");
                    return Err(err);
                }
            }
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

// ── Test-only helpers ──
//
// `NoopQuotaChecker` is used by the integration test harness so tests that
// don't care about quota don't need to wire repos. `DenyQuotaChecker` is used
// by tests that want to verify "what happens when over limit" without
// standing up the full counter/billing stack. Gated behind the
// `test-harness` feature so they don't show up in production builds.
#[cfg(any(test, feature = "test-harness"))]
pub struct NoopQuotaChecker;

#[cfg(any(test, feature = "test-harness"))]
#[async_trait]
impl QuotaChecker for NoopQuotaChecker {
    async fn check(&self, _tenant_id: &ObjectId, _resource: Resource) -> Result<(), QuotaError> {
        Ok(())
    }
}

#[cfg(any(test, feature = "test-harness"))]
pub struct DenyQuotaChecker {
    pub limit: u64,
}

#[cfg(any(test, feature = "test-harness"))]
#[async_trait]
impl QuotaChecker for DenyQuotaChecker {
    async fn check(&self, _tenant_id: &ObjectId, resource: Resource) -> Result<(), QuotaError> {
        Err(QuotaError::Exceeded {
            resource,
            limit: self.limit,
            current: self.limit,
        })
    }
}

#[cfg(test)]
mod tests {
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
        let (q, id, counts, _) =
            setup_with_plan_mode(PlanTier::Free, EnforcementMode::Enforce).await;
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
}
