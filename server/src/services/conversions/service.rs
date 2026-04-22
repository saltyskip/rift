use std::sync::Arc;

use mongodb::bson::{oid::ObjectId, DateTime};

use super::models::{ConversionEvent, ConversionMeta, IngestResult, Source};
use super::parsers::ParsedConversion;
use super::repo::ConversionsRepository;
use crate::core::webhook_dispatcher::{ConversionEventPayload, WebhookDispatcher};
use crate::services::billing::service::BillingService;
use crate::services::links::repo::LinksRepository;

/// Orchestration layer for conversion ingestion. Keeps route handlers thin per
/// CLAUDE.md's "thin transport" rule — the handler is ~15 lines (look up source,
/// parse, delegate here, return status).
pub struct ConversionsService {
    conversions_repo: Arc<dyn ConversionsRepository>,
    links_repo: Arc<dyn LinksRepository>,
    webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
    billing: Option<Arc<BillingService>>,
}

impl ConversionsService {
    pub fn new(
        conversions_repo: Arc<dyn ConversionsRepository>,
        links_repo: Arc<dyn LinksRepository>,
        webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
        billing: Option<Arc<BillingService>>,
    ) -> Self {
        Self {
            conversions_repo,
            links_repo,
            webhook_dispatcher,
            billing,
        }
    }

    /// Ingest events from a webhook source. Resolves tenant from the source.
    pub async fn ingest_parsed(
        &self,
        source: &Source,
        parsed: Vec<ParsedConversion>,
    ) -> IngestResult {
        self.ingest(source.tenant_id, source.id, parsed).await
    }

    /// Ingest events from the SDK endpoint. Tenant comes from the auth middleware,
    /// source_id is synthetic (the SDK is not a source — it's a direct channel).
    pub async fn ingest_sdk_event(
        &self,
        tenant_id: ObjectId,
        parsed: Vec<ParsedConversion>,
    ) -> IngestResult {
        // Use a zero ObjectId as a sentinel for "came from SDK, not a source."
        // This is stored in meta.source_id on the conversion event for provenance
        // but is not looked up as a real source document.
        let sdk_source_id = ObjectId::from_bytes([0u8; 12]);
        self.ingest(tenant_id, sdk_source_id, parsed).await
    }

    /// Core ingestion: dedup, attribute, store, fan out.
    async fn ingest(
        &self,
        tenant_id: ObjectId,
        source_id: ObjectId,
        parsed: Vec<ParsedConversion>,
    ) -> IngestResult {
        let mut result = IngestResult::default();

        for event in parsed {
            // 1. Idempotency check — silently drop duplicates (caller's retry logic stays happy).
            if let Some(key) = &event.idempotency_key {
                match self
                    .conversions_repo
                    .check_and_insert_dedup(&tenant_id, key)
                    .await
                {
                    Ok(false) => {
                        result.deduped += 1;
                        continue;
                    }
                    Ok(true) => {}
                    Err(e) => {
                        tracing::error!(
                            source_id = %source_id,
                            key = %key,
                            error = %e,
                            "dedup insert failed; dropping event",
                        );
                        result.failed += 1;
                        continue;
                    }
                }
            }

            // 2. Attribution lookup — need user_id → Attribution → link_id.
            // Conversions without a matching attribution are counted as
            // "unattributed" and dropped (not counted in stats).
            let link_id = match &event.user_id {
                Some(uid) => self
                    .links_repo
                    .find_attribution_by_user(&tenant_id, uid)
                    .await
                    .ok()
                    .flatten()
                    .map(|a| a.link_id),
                None => None,
            };
            let Some(link_id) = link_id else {
                tracing::debug!(
                    source_id = %source_id,
                    user_id = ?event.user_id,
                    "conversion has no matching attribution; skipping",
                );
                result.unattributed += 1;
                continue;
            };

            // 3. Insert event.
            let retention_bucket = match &self.billing {
                Some(b) => b.retention_bucket_for_tenant(&tenant_id).await.to_string(),
                None => "30d".to_string(),
            };
            let record = ConversionEvent {
                meta: ConversionMeta {
                    tenant_id,
                    link_id: link_id.clone(),
                    source_id,
                    conversion_type: event.conversion_type.clone(),
                    retention_bucket,
                },
                occurred_at: event.occurred_at.unwrap_or_else(DateTime::now),
                user_id: event.user_id.clone(),
                idempotency_key: event.idempotency_key.clone(),
                metadata: event.metadata.clone(),
            };
            let event_id = match self.conversions_repo.insert_conversion_event(record).await {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!(
                        source_id = %source_id,
                        error = %e,
                        "conversion event insert failed",
                    );
                    result.failed += 1;
                    continue;
                }
            };

            // 4. Outbound webhook (fire-and-forget; see dispatcher for retry policy).
            if let Some(dispatcher) = &self.webhook_dispatcher {
                let metadata_json = event
                    .metadata
                    .as_ref()
                    .and_then(|doc| serde_json::to_value(doc).ok());

                dispatcher.dispatch_conversion(ConversionEventPayload {
                    event_id: event_id.to_hex(),
                    tenant_id: tenant_id.to_hex(),
                    source_id: source_id.to_hex(),
                    link_id: link_id.clone(),
                    conversion_type: event.conversion_type.clone(),
                    user_id: event.user_id.clone(),
                    metadata: metadata_json,
                    timestamp: DateTime::now().try_to_rfc3339_string().unwrap_or_default(),
                });
            }

            result.accepted += 1;
        }

        result
    }
}
