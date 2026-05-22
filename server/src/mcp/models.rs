//! MCP tool input types.
//!
//! **Sharing policy** — data shapes and request bodies are shared with the REST API
//! via `#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]` on the core
//! `services/<domain>/models.rs` types. The types in this file are *transport
//! envelopes*: things that exist only at the MCP boundary, like pagination
//! parameters, resource IDs (REST puts them in URL paths; MCP needs them in the
//! tool input), and tool-specific wrappers. When an envelope needs to carry a
//! shared body, prefer `#[serde(flatten)]` over re-declaring fields, so a single
//! source of truth in `services/` stays compiler-enforced.
//!
//! See `CLAUDE.md` → "Shared model layer" for the rule.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::services::conversions::models::SourceType;
use crate::services::links::models::UpdateLinkRequest;

/// Input for the `get_link` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLinkInput {
    /// The link ID to retrieve.
    pub link_id: String,
}

/// Input for the `list_links` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListLinksInput {
    /// Maximum number of links to return (1-100, default 50).
    pub limit: Option<i64>,
    /// Cursor from a previous response for pagination.
    pub cursor: Option<String>,
}

/// Input for the `update_link` MCP tool.
///
/// `link_id` is the resource identifier (REST puts this in the URL path, MCP
/// carries it as a tool input field). The remaining fields come from the shared
/// `UpdateLinkRequest` via `#[serde(flatten)]` — `null` clears a nullable field,
/// omitting it leaves it unchanged. This is the same semantics as the REST API.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateLinkInput {
    /// The link ID to update.
    pub link_id: String,
    #[serde(flatten)]
    pub body: UpdateLinkRequest,
}

/// Input for the `delete_link` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteLinkInput {
    /// The link ID to delete.
    pub link_id: String,
}

/// Input for the `get_link_stats` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLinkStatsInput {
    /// The link ID to fetch stats for.
    pub link_id: String,
}

/// Input for the `create_source` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateSourceInput {
    /// Human-readable name, unique per tenant (1-64 chars).
    pub name: String,
    /// Source type — currently only "custom" is supported. Future integrations
    /// (RevenueCat, Stripe, etc.) will add more variants.
    pub source_type: String,
}

/// Input for the `list_sources` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSourcesInput {}

// ── Output types ──
//
// MCP tools return structured JSON via `rmcp::handler::server::wrapper::Json<T>`,
// which auto-generates an `output_schema` from `T`'s `JsonSchema`. Shared service
// response types live in `services/<domain>/models.rs` and are reused directly.
// The types below are MCP-specific (e.g. delete acknowledgement, sources with
// derived `webhook_url`) and have no REST counterpart.

/// Output for the `links.delete` MCP tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DeleteLinkOutput {
    /// The link ID that was deleted.
    pub link_id: String,
    /// Always `true` on a successful delete.
    pub deleted: bool,
}

/// Output for the `sources.create` MCP tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateSourceOutput {
    /// The newly-created source's ID (hex-encoded ObjectId).
    pub id: String,
    /// Human-readable name as supplied by the caller.
    pub name: String,
    /// Source type — currently always `custom`.
    pub source_type: SourceType,
    /// Webhook URL the customer's backend POSTs conversion events to.
    pub webhook_url: String,
}

/// One conversion source with its derived webhook URL.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SourceSummary {
    /// Source ID (hex-encoded ObjectId).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Source type.
    pub source_type: SourceType,
    /// Webhook URL the customer's backend POSTs conversion events to.
    pub webhook_url: String,
    /// When the source was created (RFC 3339).
    pub created_at: String,
}

/// Output for the `sources.list` MCP tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListSourcesOutput {
    /// All conversion sources for the authenticated tenant.
    pub sources: Vec<SourceSummary>,
}
