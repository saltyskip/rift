use async_trait::async_trait;
use mongodb::bson::{self, oid::ObjectId};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

// ── Plan / billing enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    #[default]
    Free,
    Pro,
    Business,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BillingMethod {
    #[default]
    Free,
    Stripe,
    /// Reserved for Plan B (agent lane). Not written by Plan A code.
    X402,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    #[default]
    Active,
    PastDue,
    Canceled,
}

// ── Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub monthly_quota: i64,
    pub created_at: bson::DateTime,

    #[serde(default)]
    pub plan_tier: PlanTier,
    #[serde(default)]
    pub billing_method: BillingMethod,
    #[serde(default)]
    pub status: SubscriptionStatus,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<bson::DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<bson::DateTime>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_subscription_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_tier: Option<PlanTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_until: Option<bson::DateTime>,
}

impl Default for TenantDoc {
    fn default() -> Self {
        Self {
            id: None,
            monthly_quota: 100,
            created_at: bson::DateTime::now(),
            plan_tier: PlanTier::Free,
            billing_method: BillingMethod::Free,
            status: SubscriptionStatus::Active,
            current_period_start: None,
            current_period_end: None,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            comp_tier: None,
            comp_until: None,
        }
    }
}

// ── Trait ──

#[async_trait]
pub trait TenantsRepository: Send + Sync {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct TenantsRepo {
    tenants: Collection<TenantDoc>,
}

impl TenantsRepo {
    pub async fn new(database: &Database) -> Self {
        let tenants = database.collection::<TenantDoc>("tenants");
        TenantsRepo { tenants }
    }
}

#[async_trait]
impl TenantsRepository for TenantsRepo {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String> {
        self.tenants
            .insert_one(doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
