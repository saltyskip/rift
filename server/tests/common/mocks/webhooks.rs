use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::core::webhook_dispatcher::{
    AttributionEventPayload, ClickEventPayload, ConversionEventPayload, IdentifyEventPayload,
    WebhookDispatcher,
};
use rift::services::webhooks::models::Webhook;
use rift::services::webhooks::models::WebhookEventType;
use rift::services::webhooks::repo::WebhooksRepository;

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

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        Ok(self
            .webhooks
            .lock()
            .unwrap()
            .iter()
            .filter(|w| &w.tenant_id == tenant_id)
            .count() as u64)
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

    async fn update_webhook(
        &self,
        tenant_id: &ObjectId,
        webhook_id: &ObjectId,
        active: Option<bool>,
        events: Option<Vec<WebhookEventType>>,
        url: Option<String>,
    ) -> Result<bool, String> {
        let mut webhooks = self.webhooks.lock().unwrap();
        match webhooks
            .iter_mut()
            .find(|w| &w.tenant_id == tenant_id && &w.id == webhook_id)
        {
            Some(w) => {
                if let Some(a) = active {
                    w.active = a;
                }
                if let Some(e) = events {
                    w.events = e;
                }
                if let Some(u) = url {
                    w.url = u;
                }
                Ok(true)
            }
            None => Ok(false),
        }
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
    pub conversion_payloads: Mutex<Vec<ConversionEventPayload>>,
    pub identify_payloads: Mutex<Vec<IdentifyEventPayload>>,
}

impl WebhookDispatcher for MockWebhookDispatcher {
    fn dispatch_click(&self, payload: ClickEventPayload) {
        self.click_payloads.lock().unwrap().push(payload);
    }

    fn dispatch_attribution(&self, payload: AttributionEventPayload) {
        self.attribution_payloads.lock().unwrap().push(payload);
    }

    fn dispatch_conversion(&self, payload: ConversionEventPayload) {
        self.conversion_payloads.lock().unwrap().push(payload);
    }

    fn dispatch_identify(&self, payload: IdentifyEventPayload) {
        self.identify_payloads.lock().unwrap().push(payload);
    }
}
