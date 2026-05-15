use std::sync::Arc;

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use super::models::{Webhook, WebhookEventType};
use super::repo::WebhooksRepository;
use crate::core::webhook_dispatcher::{
    AttributeEventPayload, ClickEventPayload, ConversionEventPayload, IdentifyEventPayload,
    WebhookDispatcher, WebhookPayload,
};

type HmacSha256 = Hmac<Sha256>;

crate::impl_container!(RiftWebhookDispatcher);
pub struct RiftWebhookDispatcher {
    repo: Arc<dyn WebhooksRepository>,
    http: reqwest::Client,
}

impl RiftWebhookDispatcher {
    pub fn new(repo: Arc<dyn WebhooksRepository>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { repo, http }
    }

    fn dispatch_event<T: Serialize + Send + 'static>(
        &self,
        event_type: WebhookEventType,
        event_name: &'static str,
        tenant_id: String,
        timestamp: String,
        payload: T,
    ) {
        let repo = self.repo.clone();
        let http = self.http.clone();

        tokio::spawn(async move {
            let tenant_oid = match mongodb::bson::oid::ObjectId::parse_str(&tenant_id) {
                Ok(oid) => oid,
                Err(_) => return,
            };

            // Short-TTL cache on webhook lookups — kills the per-event DB query
            // hot path when a tenant fires many conversions back-to-back.
            let webhooks =
                match cached_find_active_for_event(repo.clone(), tenant_oid, event_type.clone())
                    .await
                {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!(error = %e, event = event_name, "Failed to find webhooks");
                        return;
                    }
                };

            let data = match serde_json::to_value(&payload) {
                Ok(v) => v,
                Err(_) => return,
            };

            let envelope = WebhookPayload {
                event: event_name.to_string(),
                timestamp,
                data,
            };

            let body = match serde_json::to_string(&envelope) {
                Ok(b) => b,
                Err(_) => return,
            };

            let futures: Vec<_> = webhooks
                .iter()
                .map(|webhook| {
                    let signature = compute_hmac(&webhook.secret, &body);
                    let url = webhook.url.clone();
                    let body = body.clone();
                    let http = http.clone();
                    async move {
                        deliver_with_retry(&http, &url, &body, &signature).await;
                    }
                })
                .collect();

            futures::future::join_all(futures).await;
        });
    }
}

impl WebhookDispatcher for RiftWebhookDispatcher {
    fn dispatch_click(&self, payload: ClickEventPayload) {
        let tenant_id = payload.tenant_id.clone();
        let timestamp = payload.timestamp.clone();
        self.dispatch_event(
            WebhookEventType::Click,
            "click",
            tenant_id,
            timestamp,
            payload,
        );
    }

    fn dispatch_attribute(&self, payload: AttributeEventPayload) {
        let tenant_id = payload.tenant_id.clone();
        let timestamp = payload.timestamp.clone();
        self.dispatch_event(
            WebhookEventType::Attribute,
            "attribute",
            tenant_id,
            timestamp,
            payload,
        );
    }

    fn dispatch_conversion(&self, payload: ConversionEventPayload) {
        let tenant_id = payload.tenant_id.clone();
        let timestamp = payload.timestamp.clone();
        self.dispatch_event(
            WebhookEventType::Conversion,
            "conversion",
            tenant_id,
            timestamp,
            payload,
        );
    }

    fn dispatch_identify(&self, payload: IdentifyEventPayload) {
        let tenant_id = payload.tenant_id.clone();
        let timestamp = payload.timestamp.clone();
        self.dispatch_event(
            WebhookEventType::Identify,
            "identify",
            tenant_id,
            timestamp,
            payload,
        );
    }
}

pub(crate) fn compute_hmac(secret: &str, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ── Helpers ──

// Cached webhook lookup. Short-TTL cache keyed on (tenant_id_hex,
// event_type). Invalidation is purely time-based — 60 seconds of
// staleness is an acceptable trade for killing the per-event DB query
// hot path. A webhook newly created or toggled off will start or stop
// receiving events within 60 seconds.
//
// The cache is in-process and per-instance; multiple server instances
// each have their own cache, which is fine because the underlying
// query is cheap and the cache is purely an optimization.
//
// `Err` results are NOT cached (via `result = true`) so transient DB
// errors are retried on the next event rather than stuck for a minute.
#[cached::proc_macro::cached(
    ty = "cached::TimedCache<(String, String), Vec<Webhook>>",
    create = "{ cached::TimedCache::with_lifespan(60) }",
    convert = r#"{ (tenant_oid.to_hex(), format!("{:?}", event_type)) }"#,
    result = true
)]
async fn cached_find_active_for_event(
    repo: Arc<dyn WebhooksRepository>,
    tenant_oid: mongodb::bson::oid::ObjectId,
    event_type: WebhookEventType,
) -> Result<Vec<Webhook>, String> {
    repo.find_active_for_event(&tenant_oid, &event_type).await
}

async fn deliver_with_retry(http: &reqwest::Client, url: &str, body: &str, signature: &str) {
    let delays = [0, 1, 5, 25];
    for (attempt, delay_secs) in delays.iter().enumerate() {
        if *delay_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(*delay_secs)).await;
        }

        let result = http
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Rift-Signature", signature)
            .body(body.to_string())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => return,
            Ok(resp) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    status = %resp.status(),
                    url = url,
                    "Webhook delivery failed"
                );
            }
            Err(e) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    error = %e,
                    url = url,
                    "Webhook delivery error"
                );
            }
        }
    }
    tracing::error!(url = url, "Webhook delivery failed after all retries");
}
