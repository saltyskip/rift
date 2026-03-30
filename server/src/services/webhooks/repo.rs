use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::{Webhook, WebhookEventType};

#[async_trait]
pub trait WebhooksRepository: Send + Sync {
    async fn create_webhook(&self, webhook: Webhook) -> Result<(), String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Webhook>, String>;
    async fn delete_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
    ) -> Result<bool, String>;
    async fn set_active(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
        active: bool,
    ) -> Result<bool, String>;
    async fn find_active_for_event(
        &self,
        tenant_id: &ObjectId,
        event_type: &WebhookEventType,
    ) -> Result<Vec<Webhook>, String>;
}

#[derive(Clone)]
pub struct WebhooksRepo {
    webhooks: Collection<Webhook>,
}

impl WebhooksRepo {
    pub async fn new(database: &Database) -> Self {
        let webhooks = database.collection::<Webhook>("webhooks");

        ensure_index!(webhooks, doc! { "tenant_id": 1 }, "webhooks_tenant");
        ensure_index!(
            webhooks,
            doc! { "tenant_id": 1, "active": 1, "events": 1 },
            "webhooks_tenant_active_events"
        );

        WebhooksRepo { webhooks }
    }
}

#[async_trait]
impl WebhooksRepository for WebhooksRepo {
    async fn create_webhook(&self, webhook: Webhook) -> Result<(), String> {
        self.webhooks
            .insert_one(&webhook)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Webhook>, String> {
        let mut cursor = self
            .webhooks
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut webhooks = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            webhooks.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(webhooks)
    }

    async fn delete_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
    ) -> Result<bool, String> {
        let result = self
            .webhooks
            .delete_one(doc! { "_id": webhook_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }

    async fn set_active(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
        active: bool,
    ) -> Result<bool, String> {
        let result = self
            .webhooks
            .update_one(
                doc! { "_id": webhook_id, "tenant_id": tenant_id },
                doc! { "$set": { "active": active } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.matched_count > 0)
    }

    async fn find_active_for_event(
        &self,
        tenant_id: &ObjectId,
        event_type: &WebhookEventType,
    ) -> Result<Vec<Webhook>, String> {
        let event_str = serde_json::to_value(event_type)
            .map_err(|e| e.to_string())?
            .as_str()
            .unwrap_or_default()
            .to_string();

        let mut cursor = self
            .webhooks
            .find(doc! {
                "tenant_id": tenant_id,
                "active": true,
                "events": &event_str,
            })
            .await
            .map_err(|e| e.to_string())?;

        let mut webhooks = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            webhooks.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(webhooks)
    }
}
