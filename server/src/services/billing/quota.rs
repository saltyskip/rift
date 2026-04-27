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
/// (always Ok) or `DenyQuotaChecker` (configurable rejection) — both
/// defined later in this file and gated behind the `test-harness` feature.
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
    CreateAffiliate,
}

impl Resource {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CreateLink => "create_link",
            Self::TrackEvent => "track_event",
            Self::CreateDomain => "create_domain",
            Self::InviteTeamMember => "invite_team_member",
            Self::CreateWebhook => "create_webhook",
            Self::CreateAffiliate => "create_affiliate",
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
#[path = "quota_tests.rs"]
mod tests;
