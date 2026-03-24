use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::api::webhooks::models::Webhook;
use rift::api::webhooks::models::WebhookEventType;
use rift::api::webhooks::repo::WebhooksRepository;
use rift::core::webhook_dispatcher::{
    AttributionEventPayload, ClickEventPayload, WebhookDispatcher,
};

#[derive(Default)]
pub struct MockWebhooksRepo {
    pub webhooks: Mutex<Vec<Webhook>>,
}

#[async_trait]
impl WebhooksRepository for MockWebhooksRepo {
    async fn create_webhook(&self, webhook: Webhook) -> Result<(), String> {
        self.webhooks.lock().unwrap().push(webhook);
        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Webhook>, String> {
        Ok(self
            .webhooks
            .lock()
            .unwrap()
            .iter()
            .filter(|w| &w.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn delete_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
    ) -> Result<bool, String> {
        let mut webhooks = self.webhooks.lock().unwrap();
        let len_before = webhooks.len();
        webhooks.retain(|w| !(&w.tenant_id == tenant_id && &w.id == webhook_id));
        Ok(webhooks.len() < len_before)
    }

    async fn find_active_for_event(
        &self,
        tenant_id: &ObjectId,
        event_type: &WebhookEventType,
    ) -> Result<Vec<Webhook>, String> {
        Ok(self
            .webhooks
            .lock()
            .unwrap()
            .iter()
            .filter(|w| &w.tenant_id == tenant_id && w.active && w.events.contains(event_type))
            .cloned()
            .collect())
    }
}

#[derive(Default)]
pub struct MockWebhookDispatcher {
    pub click_payloads: Mutex<Vec<ClickEventPayload>>,
    pub attribution_payloads: Mutex<Vec<AttributionEventPayload>>,
}

impl WebhookDispatcher for MockWebhookDispatcher {
    fn dispatch_click(&self, payload: ClickEventPayload) {
        self.click_payloads.lock().unwrap().push(payload);
    }

    fn dispatch_attribution(&self, payload: AttributionEventPayload) {
        self.attribution_payloads.lock().unwrap().push(payload);
    }
}
