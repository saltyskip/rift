use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::{Affiliate, AffiliateStatus};

#[async_trait]
pub trait AffiliatesRepository: Send + Sync {
    async fn create_affiliate(&self, affiliate: &Affiliate) -> Result<(), String>;
    async fn get_by_id(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<Option<Affiliate>, String>;
    async fn find_by_partner_key(
        &self,
        tenant_id: &ObjectId,
        partner_key: &str,
    ) -> Result<Option<Affiliate>, String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Affiliate>, String>;
    /// Total affiliates on this tenant — feeds the `CreateAffiliate` quota.
    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String>;
    /// Apply optional updates. Returns `Ok(true)` if a row was touched,
    /// `Ok(false)` if no affiliate matched. Always bumps `updated_at`.
    async fn update_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
        name: Option<&str>,
        status: Option<AffiliateStatus>,
        now: DateTime,
    ) -> Result<bool, String>;
    async fn delete_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<bool, String>;
}

#[derive(Clone)]
pub struct AffiliatesRepo {
    affiliates: Collection<Affiliate>,
}

impl AffiliatesRepo {
    pub async fn new(database: &Database) -> Self {
        let affiliates = database.collection::<Affiliate>("affiliates");

        ensure_index!(affiliates, doc! { "tenant_id": 1 }, "affiliates_tenant");
        ensure_index!(
            affiliates,
            doc! { "tenant_id": 1, "partner_key": 1 },
            IndexOptions::builder().unique(true).build(),
            "affiliate_tenant_partner_key_unique"
        );

        AffiliatesRepo { affiliates }
    }
}

#[async_trait]
impl AffiliatesRepository for AffiliatesRepo {
    async fn create_affiliate(&self, affiliate: &Affiliate) -> Result<(), String> {
        self.affiliates
            .insert_one(affiliate)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn get_by_id(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<Option<Affiliate>, String> {
        self.affiliates
            .find_one(doc! { "_id": affiliate_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_partner_key(
        &self,
        tenant_id: &ObjectId,
        partner_key: &str,
    ) -> Result<Option<Affiliate>, String> {
        self.affiliates
            .find_one(doc! { "tenant_id": tenant_id, "partner_key": partner_key })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Affiliate>, String> {
        let mut cursor = self
            .affiliates
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut affiliates = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            affiliates.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(affiliates)
    }

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        self.affiliates
            .count_documents(doc! { "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn update_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
        name: Option<&str>,
        status: Option<AffiliateStatus>,
        now: DateTime,
    ) -> Result<bool, String> {
        let mut set = doc! { "updated_at": now };
        if let Some(n) = name {
            set.insert("name", n);
        }
        if let Some(s) = status {
            let s_str = match s {
                AffiliateStatus::Active => "active",
                AffiliateStatus::Disabled => "disabled",
            };
            set.insert("status", s_str);
        }

        let result = self
            .affiliates
            .update_one(
                doc! { "_id": affiliate_id, "tenant_id": tenant_id },
                doc! { "$set": set },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.matched_count > 0)
    }

    async fn delete_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<bool, String> {
        let result = self
            .affiliates
            .delete_one(doc! { "_id": affiliate_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }
}
