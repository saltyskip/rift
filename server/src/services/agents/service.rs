use std::sync::Arc;

use mongodb::bson::DateTime;

use super::models::{
    AgentActionEvent, AgentActionMeta, AgentError, RecordActionRequest, RecordActionResponse,
};
use super::repo::AgentActionsRepository;
use crate::core::public_id::{AgentActionId, JourneyToken, TenantId};
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::billing::service::TierResolver;

crate::impl_container!(AgentsService);
/// Orchestration for agent-action ingestion. Keeps route handlers thin per
/// CLAUDE.md's transport rule: the handler resolves the tenant from auth and
/// delegates here. Quota enforcement lives in this service layer (not the
/// route) so any future transport (e.g. an MCP tool) inherits it.
pub struct AgentsService {
    repo: Arc<dyn AgentActionsRepository>,
    /// Retention tier resolver — freezes the TTL bucket on each event. Optional
    /// for reduced-feature builds, mirroring `ConversionsService`.
    tiers: Option<Arc<dyn TierResolver>>,
    /// Usage/quota gate. v0 reuses the `TrackEvent` meter (same counter as
    /// clicks/conversions); split into its own meter when pricing is decided.
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl AgentsService {
    pub fn new(
        repo: Arc<dyn AgentActionsRepository>,
        tiers: Option<Arc<dyn TierResolver>>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self { repo, tiers, quota }
    }

    /// Record one instrumented tool call. Returns the stored event's public id
    /// and, when requested, a freshly minted journey token for a handoff link.
    pub async fn record_action(
        &self,
        tenant_id: &TenantId,
        req: RecordActionRequest,
    ) -> Result<RecordActionResponse, AgentError> {
        // Quota first — an over-limit tenant's action is rejected before any write.
        if let Some(q) = &self.quota {
            q.check(tenant_id, Resource::TrackEvent)
                .await
                .map_err(AgentError::QuotaExceeded)?;
        }

        let retention_bucket = match &self.tiers {
            Some(t) => t.retention_bucket_for_tenant(tenant_id).await.to_string(),
            None => "30d".to_string(),
        };

        let journey_token = req.mint_journey_token.then(JourneyToken::new);
        let agent_action_id = AgentActionId::new();

        let event = AgentActionEvent {
            id: Some(agent_action_id),
            meta: AgentActionMeta {
                tenant_id: *tenant_id,
                agent_platform: req.agent_platform,
                retention_bucket,
            },
            occurred_at: DateTime::now(),
            tool: req.tool,
            status: req.status,
            latency_ms: req.latency_ms,
            intent: req.intent,
            journey_token,
        };

        self.repo
            .insert_action(event)
            .await
            .map_err(AgentError::Storage)?;

        Ok(RecordActionResponse {
            agent_action_id,
            journey_token,
        })
    }
}
