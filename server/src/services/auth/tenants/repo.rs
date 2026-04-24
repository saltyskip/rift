use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

// ── Plan / billing enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    #[default]
    Free,
    Pro,
    Business,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingMethod {
    #[default]
    Free,
    Stripe,
    /// Reserved for Plan B (agent lane). Not written by Plan A code.
    X402,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
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

/// Fields a Stripe subscription event updates atomically. Pass `None` for
/// fields the caller doesn't want to touch; `Some(value)` replaces.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionUpdate {
    pub plan_tier: Option<PlanTier>,
    pub billing_method: Option<BillingMethod>,
    pub status: Option<SubscriptionStatus>,
    pub current_period_start: Option<bson::DateTime>,
    pub current_period_end: Option<bson::DateTime>,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
}

#[async_trait]
pub trait TenantsRepository: Send + Sync {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String>;
    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<TenantDoc>, String>;

    /// Resolve a tenant by the Stripe customer id stored on prior subscription
    /// events. Used by webhook handlers that receive `customer.subscription.*`
    /// events lacking a `client_reference_id`.
    async fn find_by_stripe_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<TenantDoc>, String>;

    /// Find the tenant whose owner user has the given email. Used by the
    /// magic-link resolver to decide between creating a new customer and
    /// upgrading/managing an existing tenant's subscription.
    async fn find_by_owner_email(&self, email: &str) -> Result<Option<TenantDoc>, String>;

    /// Apply a partial subscription update. `Some(_)` fields are set;
    /// `None` fields are left untouched.
    async fn apply_subscription_update(
        &self,
        tenant_id: &ObjectId,
        update: SubscriptionUpdate,
    ) -> Result<bool, String>;

    /// End-of-subscription path for `customer.subscription.deleted`. Drops
    /// the tenant back to Free and clears Stripe identifiers.
    async fn clear_subscription(&self, tenant_id: &ObjectId) -> Result<bool, String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct TenantsRepo {
    tenants: Collection<TenantDoc>,
    users: Collection<bson::Document>,
}

impl TenantsRepo {
    pub async fn new(database: &Database) -> Self {
        let tenants = database.collection::<TenantDoc>("tenants");
        let users = database.collection::<bson::Document>("users");
        TenantsRepo { tenants, users }
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

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<TenantDoc>, String> {
        self.tenants
            .find_one(doc! { "_id": id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_stripe_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<TenantDoc>, String> {
        self.tenants
            .find_one(doc! { "stripe_customer_id": customer_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_owner_email(&self, email: &str) -> Result<Option<TenantDoc>, String> {
        // Two-hop lookup: users collection is authoritative for email→tenant.
        // We pick the owner row if one exists; otherwise fall back to any
        // verified user on the tenant (covers tenants whose "owner" flag was
        // lost during earlier migrations).
        let user_doc = self
            .users
            .find_one(doc! {
                "email": email,
                "is_owner": true,
            })
            .await
            .map_err(|e| e.to_string())?;
        let Some(user_doc) = user_doc else {
            return Ok(None);
        };
        let Some(tenant_id) = user_doc.get_object_id("tenant_id").ok() else {
            return Ok(None);
        };
        self.tenants
            .find_one(doc! { "_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn apply_subscription_update(
        &self,
        tenant_id: &ObjectId,
        update: SubscriptionUpdate,
    ) -> Result<bool, String> {
        let mut set_doc = mongodb::bson::Document::new();
        if let Some(tier) = update.plan_tier {
            set_doc.insert(
                "plan_tier",
                bson::to_bson(&tier).map_err(|e| e.to_string())?,
            );
        }
        if let Some(method) = update.billing_method {
            set_doc.insert(
                "billing_method",
                bson::to_bson(&method).map_err(|e| e.to_string())?,
            );
        }
        if let Some(status) = update.status {
            set_doc.insert("status", bson::to_bson(&status).map_err(|e| e.to_string())?);
        }
        if let Some(start) = update.current_period_start {
            set_doc.insert("current_period_start", start);
        }
        if let Some(end) = update.current_period_end {
            set_doc.insert("current_period_end", end);
        }
        if let Some(cust) = update.stripe_customer_id {
            set_doc.insert("stripe_customer_id", cust);
        }
        if let Some(sub) = update.stripe_subscription_id {
            set_doc.insert("stripe_subscription_id", sub);
        }
        if set_doc.is_empty() {
            return Ok(true);
        }
        let result = self
            .tenants
            .update_one(doc! { "_id": tenant_id }, doc! { "$set": set_doc })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.matched_count == 1)
    }

    async fn clear_subscription(&self, tenant_id: &ObjectId) -> Result<bool, String> {
        let update = doc! {
            "$set": {
                "plan_tier": bson::to_bson(&PlanTier::Free).map_err(|e| e.to_string())?,
                "billing_method": bson::to_bson(&BillingMethod::Free).map_err(|e| e.to_string())?,
                "status": bson::to_bson(&SubscriptionStatus::Canceled).map_err(|e| e.to_string())?,
            },
            "$unset": {
                "current_period_start": "",
                "current_period_end": "",
                "stripe_subscription_id": "",
            }
        };
        let result = self
            .tenants
            .update_one(doc! { "_id": tenant_id }, update)
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.matched_count == 1)
    }
}
