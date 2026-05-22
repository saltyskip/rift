use std::sync::Arc;

use mongodb::bson::{oid::ObjectId, DateTime};

use super::models::ParsedConversion;
use super::models::{ConversionEvent, ConversionMeta, IngestResult, Source};
use super::repo::ConversionsRepository;
use crate::core::webhook_dispatcher::{ConversionEventPayload, WebhookDispatcher};
use crate::services::app_users::repo::AppUsersRepository;
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::billing::service::TierResolver;

crate::impl_container!(ConversionsService);
/// Orchestration layer for conversion ingestion. Keeps route handlers thin per
/// CLAUDE.md's "thin transport" rule — the handler is ~15 lines (look up source,
/// parse, delegate here, return status).
pub struct ConversionsService {
    conversions_repo: Arc<dyn ConversionsRepository>,
    /// User-existence check for attribution. Optional during the cutover
    /// so reduced-feature builds keep booting; ingest treats `None` as
    /// "trust the caller" and accepts every event with a user_id.
    app_users_repo: Option<Arc<dyn AppUsersRepository>>,
    webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
    tiers: Option<Arc<dyn TierResolver>>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl ConversionsService {
    pub fn new(
        conversions_repo: Arc<dyn ConversionsRepository>,
        app_users_repo: Option<Arc<dyn AppUsersRepository>>,
        webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
        tiers: Option<Arc<dyn TierResolver>>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self {
            conversions_repo,
            app_users_repo,
            webhook_dispatcher,
            tiers,
            quota,
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

            // 2. Attribution check — require a known user. Credit is
            // computed at read time from the user's journey, so the
            // ingest path no longer needs to resolve a link_id; it just
            // confirms the user_id exists in `app_users` (or trusts
            // the caller when `app_users_repo` isn't wired).
            let Some(user_id) = event.user_id.as_ref() else {
                tracing::debug!(
                    source_id = %source_id,
                    "conversion has no user_id; skipping",
                );
                result.unattributed += 1;
                continue;
            };
            let user_known = match &self.app_users_repo {
                Some(repo) => repo
                    .find_by_user_id(&tenant_id, user_id)
                    .await
                    .ok()
                    .flatten()
                    .is_some(),
                None => true,
            };
            if !user_known {
                tracing::debug!(
                    source_id = %source_id,
                    user_id = %user_id,
                    "conversion user_id not found in app_users; skipping",
                );
                result.unattributed += 1;
                continue;
            }

            // 2b. Quota check (TrackEvent) — same "events per month" counter
            // as clicks. Runs post-dedup so a duplicate doesn't consume the
            // quota twice; runs before insert so a rejection keeps the
            // event out of the DB. In log-only mode returns Ok and records
            // a warn; in enforce mode returns Err and we count it as failed.
            if let Some(q) = &self.quota {
                if let Err(e) = q.check(&tenant_id, Resource::TrackEvent).await {
                    tracing::info!(
                        source_id = %source_id,
                        error = %e,
                        "conversion_ingest_quota_rejected"
                    );
                    result.failed += 1;
                    continue;
                }
            }

            // 3. Insert event.
            let retention_bucket = match &self.tiers {
                Some(b) => b.retention_bucket_for_tenant(&tenant_id).await.to_string(),
                None => "30d".to_string(),
            };
            let record = ConversionEvent {
                id: Some(ObjectId::new()),
                meta: ConversionMeta {
                    tenant_id,
                    // Credit is computed at read time; no link_id frozen
                    // in storage. Legacy field kept on the schema for
                    // back-compat with pre-Phase-6 rows.
                    link_id: None,
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
                    link_id: None,
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
