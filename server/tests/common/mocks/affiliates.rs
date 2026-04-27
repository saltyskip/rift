use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use std::sync::Mutex;

use rift::services::affiliates::models::{Affiliate, AffiliateStatus};
use rift::services::affiliates::repo::AffiliatesRepository;

#[derive(Default)]
pub struct MockAffiliatesRepo {
    pub affiliates: Mutex<Vec<Affiliate>>,
}

#[async_trait]
impl AffiliatesRepository for MockAffiliatesRepo {
    async fn create_affiliate(&self, affiliate: &Affiliate) -> Result<(), String> {
        let mut store = self.affiliates.lock().unwrap();
        // Mirror the unique compound index: (tenant_id, partner_key).
        if store
            .iter()
            .any(|a| a.tenant_id == affiliate.tenant_id && a.partner_key == affiliate.partner_key)
        {
            return Err("E11000 duplicate key (tenant_id, partner_key)".to_string());
        }
        store.push(affiliate.clone());
        Ok(())
    }

    async fn get_by_id(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<Option<Affiliate>, String> {
        Ok(self
            .affiliates
            .lock()
            .unwrap()
            .iter()
            .find(|a| &a.tenant_id == tenant_id && &a.id == affiliate_id)
            .cloned())
    }

    async fn find_by_partner_key(
        &self,
        tenant_id: &ObjectId,
        partner_key: &str,
    ) -> Result<Option<Affiliate>, String> {
        Ok(self
            .affiliates
            .lock()
            .unwrap()
            .iter()
            .find(|a| &a.tenant_id == tenant_id && a.partner_key == partner_key)
            .cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Affiliate>, String> {
        let mut affiliates: Vec<Affiliate> = self
            .affiliates
            .lock()
            .unwrap()
            .iter()
            .filter(|a| &a.tenant_id == tenant_id)
            .cloned()
            .collect();
        // Match production sort: created_at desc.
        affiliates.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(affiliates)
    }

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        Ok(self
            .affiliates
            .lock()
            .unwrap()
            .iter()
            .filter(|a| &a.tenant_id == tenant_id)
            .count() as u64)
    }

    async fn update_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
        name: Option<&str>,
        status: Option<AffiliateStatus>,
        now: DateTime,
    ) -> Result<bool, String> {
        let mut store = self.affiliates.lock().unwrap();
        let Some(a) = store
            .iter_mut()
            .find(|a| &a.tenant_id == tenant_id && &a.id == affiliate_id)
        else {
            return Ok(false);
        };
        if let Some(n) = name {
            a.name = n.to_string();
        }
        if let Some(s) = status {
            a.status = s;
        }
        a.updated_at = now;
        Ok(true)
    }

    async fn delete_affiliate(
        &self,
        tenant_id: &ObjectId,
        affiliate_id: &ObjectId,
    ) -> Result<bool, String> {
        let mut store = self.affiliates.lock().unwrap();
        let len = store.len();
        store.retain(|a| !(&a.tenant_id == tenant_id && &a.id == affiliate_id));
        Ok(store.len() < len)
    }
}
