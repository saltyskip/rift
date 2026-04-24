//! Billing endpoints — mirrors `server/src/api/billing/routes.rs`.

use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    Free,
    Pro,
    Business,
    Scale,
}

impl PlanTier {
    pub fn as_slug(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Business => "business",
            Self::Scale => "scale",
        }
    }

    pub fn parse_paid(s: &str) -> Option<Self> {
        match s {
            "pro" => Some(Self::Pro),
            "business" => Some(Self::Business),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingMethod {
    Free,
    Stripe,
    X402,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BillingLimits {
    pub max_links: Option<u64>,
    pub max_events_per_month: Option<u64>,
    pub max_domains: Option<u64>,
    pub max_team_members: Option<u64>,
    pub max_webhooks: Option<u64>,
    pub analytics_retention: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BillingStatus {
    pub plan_tier: PlanTier,
    pub effective_tier: PlanTier,
    pub comp_active: bool,
    pub billing_method: BillingMethod,
    pub status: SubscriptionStatus,
    pub current_period_end: Option<i64>,
    pub limits: BillingLimits,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutSession {
    pub checkout_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalSession {
    pub portal_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub status: String,
    pub current_period_end: Option<i64>,
}

impl RiftClient {
    pub async fn billing_status(&self) -> Result<BillingStatus, RiftClientError> {
        self.get("/v1/billing/status").await
    }

    pub async fn create_checkout(
        &self,
        tier: PlanTier,
    ) -> Result<CheckoutSession, RiftClientError> {
        let path = format!("/v1/billing/stripe/checkout?tier={}", tier.as_slug());
        self.post(&path, &serde_json::json!({}), false).await
    }

    pub async fn create_billing_portal(&self) -> Result<PortalSession, RiftClientError> {
        self.post("/v1/billing/stripe/portal", &serde_json::json!({}), false)
            .await
    }

    pub async fn cancel_subscription(&self) -> Result<CancelResponse, RiftClientError> {
        self.post("/v1/billing/cancel", &serde_json::json!({}), false)
            .await
    }
}
