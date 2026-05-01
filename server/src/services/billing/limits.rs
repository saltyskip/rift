use crate::services::auth::tenants::repo::PlanTier;

pub use super::models::PlanLimits;

/// Canonical limits table. Matches the pricing page committed in PR #91.
///
/// Pro is a solo tier (1 team member). Business unlocks unlimited team members
/// — this is the primary differentiator the pricing page advertises for that
/// tier upgrade.
pub fn limits_for(tier: PlanTier) -> PlanLimits {
    match tier {
        PlanTier::Free => PlanLimits {
            max_links: Some(50),
            max_events_per_month: Some(10_000),
            max_domains: Some(1),
            max_team_members: Some(1),
            max_webhooks: Some(1),
            max_affiliates: Some(1),
            retention_bucket: "30d",
        },
        PlanTier::Pro => PlanLimits {
            max_links: Some(2_000),
            max_events_per_month: Some(100_000),
            max_domains: Some(5),
            max_team_members: Some(1),
            max_webhooks: Some(5),
            max_affiliates: Some(5),
            retention_bucket: "1y",
        },
        PlanTier::Business => PlanLimits {
            max_links: Some(20_000),
            max_events_per_month: Some(500_000),
            max_domains: Some(20),
            max_team_members: None,
            max_webhooks: Some(20),
            max_affiliates: Some(20),
            retention_bucket: "3y",
        },
        PlanTier::Scale => PlanLimits {
            max_links: Some(100_000),
            max_events_per_month: Some(2_000_000),
            max_domains: None,
            max_team_members: None,
            max_webhooks: None,
            max_affiliates: None,
            retention_bucket: "5y",
        },
    }
}
