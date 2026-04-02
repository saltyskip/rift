use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Mutex;

use rift::services::auth::usage::repo::{UsageDoc, UsageRepository};

#[derive(Default)]
pub struct MockUsageRepo {
    pub usage: Mutex<Vec<UsageDoc>>,
}

#[async_trait]
impl UsageRepository for MockUsageRepo {
    async fn record_usage(&self, usage_doc: UsageDoc) {
        self.usage.lock().unwrap().push(usage_doc);
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
