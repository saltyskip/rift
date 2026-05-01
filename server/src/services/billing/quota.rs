use async_trait::async_trait;
use chrono::{Datelike, Utc};
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::limits::{limits_for, PlanLimits};
use super::models::BillingError;
use super::repos::event_counters::EventCountersRepository;
use super::service::TierResolver;

// Re-export quota data types from models.rs so existing callers (which import
// via `services::billing::quota::{...}`) keep compiling. The data types have
// moved to `models.rs` per the strict pub-types-in-models rule; the
// implementation containers (QuotaService, traits, etc.) stay here.
pub use super::models::{EnforcementMode, QuotaError, Resource};

/// The quota decision surface used by every service that creates a resource
/// or tracks an event.
///
/// Services inject `Arc<dyn QuotaChecker>` rather than the concrete
/// `QuotaService`, which keeps the internal fanout (billing, event
/// counters, per-domain resource counts) invisible at the call site.
///
/// Production wiring uses `QuotaService`; tests use `NoopQuotaChecker`
/// (always Ok) or `DenyQuotaChecker` (configurable rejection) — both
/// defined later in this file and gated behind the `test-harness` feature.
#[async_trait]
pub trait QuotaChecker: Send + Sync {
    async fn check(&self, tenant_id: &ObjectId, resource: Resource) -> Result<(), QuotaError>;

    /// Pre-check whether `n` units of `resource` would fit. Used by bulk
    /// operations (e.g. `POST /v1/links/bulk`) so the whole batch is gated
    /// by a single decision rather than failing partway through. The default
    /// impl falls back to N single-unit `check` calls — production
    /// `QuotaService` overrides with one comparison.
    async fn check_n(
        &self,
        tenant_id: &ObjectId,
        resource: Resource,
        n: u64,
    ) -> Result<(), QuotaError> {
        for _ in 0..n {
            self.check(tenant_id, resource).await?;
        }
        Ok(())
    }
}

/// Per-resource counter for "things already owned by this tenant" — used by
/// `QuotaService::check` to decide whether a new create is allowed. Each
/// existing repo already has (or gets) a `count_by_tenant` for exactly this.
#[async_trait::async_trait]
pub trait ResourceCounts: Send + Sync {
    async fn count(&self, tenant_id: &ObjectId, resource: Resource) -> Result<u64, String>;
}

crate::impl_container!(QuotaService);
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

    async fn check_n(
        &self,
        tenant_id: &ObjectId,
        resource: Resource,
        n: u64,
    ) -> Result<(), QuotaError> {
        if n == 0 {
            return Ok(());
        }
        // TrackEvent has its own atomic counter path; bulk pre-check isn't
        // a use case for it today. Fall back to per-unit checks.
        if matches!(resource, Resource::TrackEvent) {
            for _ in 0..n {
                self.check(tenant_id, resource).await?;
            }
            return Ok(());
        }

        let tier = self.billing.effective_tier(tenant_id).await?;
        let limits = limits_for(tier);
        let max = match limit_for_resource(&limits, resource) {
            Some(m) => m,
            None => return Ok(()), // unlimited
        };

        let current = self
            .resource_counts
            .count(tenant_id, resource)
            .await
            .map_err(|e| QuotaError::Billing(BillingError::Internal(e)))?;

        if current.saturating_add(n) <= max {
            return Ok(());
        }

        let err = QuotaError::Exceeded {
            resource,
            limit: max,
            current,
        };
        match self.mode {
            EnforcementMode::LogOnly => {
                tracing::warn!(
                    quota_error = %err,
                    batch_size = n,
                    mode = "log_only",
                    "quota_check_n_would_reject"
                );
                Ok(())
            }
            EnforcementMode::Enforce => {
                tracing::info!(quota_error = %err, batch_size = n, mode = "enforce", "quota_n_rejected");
                Err(err)
            }
        }
    }
}

fn limit_for_resource(limits: &PlanLimits, resource: Resource) -> Option<u64> {
    match resource {
        Resource::CreateLink => limits.max_links,
        Resource::TrackEvent => limits.max_events_per_month,
        Resource::CreateDomain => limits.max_domains,
        Resource::InviteTeamMember => limits.max_team_members,
        Resource::CreateWebhook => limits.max_webhooks,
        Resource::CreateAffiliate => limits.max_affiliates,
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
crate::impl_container!(NoopQuotaChecker);
#[cfg(any(test, feature = "test-harness"))]
pub struct NoopQuotaChecker;

#[cfg(any(test, feature = "test-harness"))]
#[async_trait]
impl QuotaChecker for NoopQuotaChecker {
    async fn check(&self, _tenant_id: &ObjectId, _resource: Resource) -> Result<(), QuotaError> {
        Ok(())
    }
}

crate::impl_container!(DenyQuotaChecker);
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
#[path = "quota_tests.rs"]
mod tests;
