use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::collections::HashSet;
use std::sync::Mutex;

use rift::services::conversions::models::{ConversionEvent, Source, SourceType};
use rift::services::conversions::repo::ConversionsRepository;

#[derive(Default)]
pub struct MockConversionsRepo {
    dedup_keys: Mutex<HashSet<(String, String)>>,
    events: Mutex<Vec<ConversionEvent>>,
}

#[async_trait]
impl ConversionsRepository for MockConversionsRepo {
    async fn create_source(
        &self,
        _tenant_id: ObjectId,
        _name: String,
        _source_type: SourceType,
    ) -> Result<Source, String> {
        unimplemented!("not needed for conversion tests")
    }

    async fn find_source_by_token(&self, _url_token: &str) -> Result<Option<Source>, String> {
        Ok(None)
    }

    async fn find_source_by_id(
        &self,
        _tenant_id: &ObjectId,
        _id: &ObjectId,
    ) -> Result<Option<Source>, String> {
        Ok(None)
    }

    async fn list_sources(&self, _tenant_id: &ObjectId) -> Result<Vec<Source>, String> {
        Ok(Vec::new())
    }

    async fn delete_source(&self, _tenant_id: &ObjectId, _id: &ObjectId) -> Result<bool, String> {
        Ok(false)
    }

    async fn get_or_create_default_custom_source(
        &self,
        _tenant_id: ObjectId,
    ) -> Result<Source, String> {
        unimplemented!("not needed for conversion tests")
    }

    async fn insert_conversion_event(&self, event: ConversionEvent) -> Result<ObjectId, String> {
        self.events.lock().unwrap().push(event);
        Ok(ObjectId::new())
    }

    async fn check_and_insert_dedup(
        &self,
        tenant_id: &ObjectId,
        idempotency_key: &str,
    ) -> Result<bool, String> {
        let key = (tenant_id.to_hex(), idempotency_key.to_string());
        let mut keys = self.dedup_keys.lock().unwrap();
        if keys.contains(&key) {
            Ok(false)
        } else {
            keys.insert(key);
            Ok(true)
        }
    }

    async fn count_by_type_for_users(
        &self,
        _tenant_id: &ObjectId,
        _user_ids: &[String],
        _from: mongodb::bson::DateTime,
        _to: mongodb::bson::DateTime,
    ) -> Result<Vec<(String, u64)>, String> {
        Ok(Vec::new())
    }
}
