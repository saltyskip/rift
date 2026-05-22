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
use serde::Deserialize;

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
