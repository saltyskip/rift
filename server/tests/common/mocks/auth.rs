use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::api::auth::repo::{ApiKeyDoc, AuthRepository, UsageDoc};

#[derive(Default)]
pub struct MockAuthRepo {
    pub keys: Mutex<Vec<ApiKeyDoc>>,
    pub usage: Mutex<Vec<UsageDoc>>,
}

#[async_trait]
impl AuthRepository for MockAuthRepo {
    async fn find_key_by_hash(&self, hash: &str) -> Option<ApiKeyDoc> {
        self.keys
            .lock()
            .unwrap()
            .iter()
            .find(|k| k.key_hash == hash && k.verified)
            .cloned()
    }

    async fn find_key_by_email(&self, email: &str) -> Option<ApiKeyDoc> {
        self.keys
            .lock()
            .unwrap()
            .iter()
            .find(|k| k.email == email)
            .cloned()
    }

    async fn upsert_key(&self, key_doc: &ApiKeyDoc) -> Result<(), String> {
        let mut keys = self.keys.lock().unwrap();
        if let Some(existing) = keys.iter_mut().find(|k| k.email == key_doc.email) {
            existing.key_hash = key_doc.key_hash.clone();
            existing.key_prefix = key_doc.key_prefix.clone();
            existing.verified = key_doc.verified;
            existing.verify_token = key_doc.verify_token.clone();
            existing.monthly_quota = key_doc.monthly_quota;
        } else {
            keys.push(key_doc.clone());
        }
        Ok(())
    }

    async fn verify_key(&self, token: &str) -> bool {
        let mut keys = self.keys.lock().unwrap();
        if let Some(key) = keys
            .iter_mut()
            .find(|k| k.verify_token.as_deref() == Some(token) && !k.verified)
        {
            key.verified = true;
            key.verify_token = None;
            true
        } else {
            false
        }
    }

    async fn record_usage(&self, usage_doc: UsageDoc) {
        self.usage.lock().unwrap().push(usage_doc);
    }

    async fn count_key_usage_since(&self, key_id: &ObjectId, _since: DateTime<Utc>) -> i64 {
        self.usage
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.api_key_id.as_ref() == Some(key_id))
            .count() as i64
    }

    async fn count_ip_usage_since(&self, ip: &str, _since: DateTime<Utc>) -> i64 {
        self.usage
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.api_key_id.is_none() && u.ip == ip)
            .count() as i64
    }
}
