//! Data types for the Agent Layer — the v0 capture primitive.
//!
//! An *agent action* is a single tool call captured on an instrumented MCP
//! server (via the Rift SDK middleware that wraps `tools/call`). Events are
//! stored in the `agent_action_events` MongoDB time series collection, the same
//! pattern as `click_events` / `conversion_events`. This is the **sensor** half
//! of the Agent Layer; attribution + the handoff rail build on top (see
//! `docs/agent-layer-spec.md`).

use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::public_id::{AgentActionId, JourneyToken, TenantId};

// ── Database document ──

/// A single agent-action event. Stored in the `agent_action_events` time series
/// collection — the source of truth. Funnel stats are computed on read via
/// aggregation pipelines (no counter cache), mirroring conversions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionEvent {
    /// Document identifier. Auto-generated on insert; round-tripped on read.
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<AgentActionId>,
    pub meta: AgentActionMeta,
    /// When the tool call happened — the time field for the time series collection.
    pub occurred_at: DateTime,
    /// Tool name as registered on the operator's MCP server, e.g. `recommend_plan`.
    pub tool: String,
    /// Outcome of the underlying tool call: `ok` | `error`.
    pub status: String,
    /// Wall-clock duration of the tool call in milliseconds (captured by the SDK).
    pub latency_ms: u32,
    /// Tool arguments captured verbatim as caller intent. Capped by the SDK,
    /// stored as-is, never indexed or queried in v0 — mirrors conversion metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<serde_json::Value>,
    /// Set when the SDK rewrote a handoff URL in the tool response into a Rift
    /// deep link; this token binds a later install/conversion back to this action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journey_token: Option<JourneyToken>,
}

/// Meta fields for the time series bucket. Fields here are efficient to
/// `$match` against (MongoDB buckets documents by meta values).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionMeta {
    pub tenant_id: TenantId,
    /// Self-reported MCP client identity (`clientInfo.name`), e.g. `chatgpt`,
    /// `claude`, `perplexity`. `None` when the client didn't announce it — common
    /// in stateless-HTTP MCP where `peer_info` is absent (see spec §9.5). The
    /// value is self-reported and not verified in v0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_platform: Option<String>,
    /// Retention bucket frozen at insert time — same per-tier TTL pattern as ClickMeta.
    #[serde(default = "crate::services::links::models::default_retention_bucket")]
    pub retention_bucket: String,
}

// ── Ingest DTOs (`POST /v1/agents/actions`) ──

/// Body the SDK posts for each instrumented tool call. Authentication (tenant
/// resolution) comes from the `rl_live_` key in the auth middleware, not the body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecordActionRequest {
    /// Tool name that was invoked.
    #[schema(example = "recommend_plan")]
    pub tool: String,
    /// Self-reported calling agent (`clientInfo.name`). Optional.
    #[schema(example = "chatgpt")]
    #[serde(default)]
    pub agent_platform: Option<String>,
    /// Tool arguments, captured as intent. Optional; capped by the SDK before send.
    #[serde(default)]
    pub intent: Option<serde_json::Value>,
    /// Outcome of the tool call. Defaults to `ok`.
    #[schema(example = "ok")]
    #[serde(default = "default_status")]
    pub status: String,
    /// Wall-clock duration of the tool call in milliseconds.
    #[serde(default)]
    pub latency_ms: u32,
    /// When true, Rift mints a single-use journey token for an agent→human
    /// handoff link and returns it; the SDK rewrites the response URL with it.
    #[serde(default)]
    pub mint_journey_token: bool,
}

/// Default for [`RecordActionRequest::status`] — a tool call is assumed
/// successful unless the SDK reports otherwise.
pub fn default_status() -> String {
    "ok".to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordActionResponse {
    /// Public id of the stored agent-action event.
    pub agent_action_id: AgentActionId,
    /// Present only when `mint_journey_token` was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journey_token: Option<JourneyToken>,
}

// ── Service-layer errors ──

/// Errors surfaced by `AgentsService`. Route handlers map `QuotaExceeded` to
/// `402 Payment Required` (via the shared billing quota_response helper) and
/// everything else to `500`.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("quota exceeded")]
    QuotaExceeded(crate::services::billing::quota::QuotaError),
    #[error("storage error: {0}")]
    Storage(String),
}
