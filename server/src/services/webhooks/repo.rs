use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::{Webhook, WebhookEventType};

#[async_trait]
pub trait WebhooksRepository: Send + Sync {
    async fn create_webhook(&self, webhook: Webhook) -> Result<(), String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Webhook>, String>;
    /// Total webhooks on this tenant — feeds the CreateWebhook quota.
    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String>;
    async fn delete_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
    ) -> Result<bool, String>;
    /// Patch one or more mutable fields. `Some` overwrites; `None` leaves
    /// the field unchanged. Returns `true` when the webhook existed for
    /// this tenant (even if no fields were actually different — the
    /// "matched but unchanged" case is success for an idempotent PATCH).
    async fn update_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
        active: Option<bool>,
        events: Option<Vec<WebhookEventType>>,
    ) -> Result<bool, String>;
    async fn find_active_for_event(
        &self,
        tenant_id: &ObjectId,
        event_type: &WebhookEventType,
    ) -> Result<Vec<Webhook>, String>;
}

crate::impl_container!(WebhooksRepo);
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

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        self.webhooks
            .count_documents(doc! { "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())
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

    async fn update_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
        active: Option<bool>,
        events: Option<Vec<WebhookEventType>>,
    ) -> Result<bool, String> {
        let mut set_doc = mongodb::bson::Document::new();
        if let Some(a) = active {
            set_doc.insert("active", a);
        }
        if let Some(e) = events {
            let bson = mongodb::bson::to_bson(&e).map_err(|err| err.to_string())?;
            set_doc.insert("events", bson);
        }
        // No-op patches still need to confirm the webhook exists so the
        // route can return 404 vs 200 correctly. Use find_one in that
        // case; otherwise issue the $set.
        if set_doc.is_empty() {
            let exists = self
                .webhooks
                .find_one(doc! { "_id": webhook_id, "tenant_id": tenant_id })
                .await
                .map_err(|e| e.to_string())?
                .is_some();
            return Ok(exists);
        }
        let result = self
            .webhooks
            .update_one(
                doc! { "_id": webhook_id, "tenant_id": tenant_id },
                doc! { "$set": set_doc },
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
