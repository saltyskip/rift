use schemars::JsonSchema;
use serde::Deserialize;

/// Input for the `create_link` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateLinkInput {
    /// Optional vanity slug (3-64 chars, alphanumeric + hyphens). Requires a verified custom domain.
    pub custom_id: Option<String>,
    /// iOS deep link URI (e.g. "myapp://product/123").
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    pub web_url: Option<String>,
    /// App Store link for iOS.
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata (campaign name, source, etc.).
    pub metadata: Option<serde_json::Value>,
    /// Context for AI agents consuming this link.
    pub agent_context: Option<AgentContextInput>,
}

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
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateLinkInput {
    /// The link ID to update.
    pub link_id: String,
    /// iOS deep link URI.
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    pub android_deep_link: Option<String>,
    /// Web fallback URL.
    pub web_url: Option<String>,
    /// App Store link for iOS.
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    pub metadata: Option<serde_json::Value>,
    /// Context for AI agents consuming this link.
    pub agent_context: Option<AgentContextInput>,
}

/// Input for the `delete_link` MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteLinkInput {
    /// The link ID to delete.
    pub link_id: String,
}

/// Agent context attached to a link (action, CTA, description).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentContextInput {
    /// The action this link performs (e.g. "open_product", "start_trial").
    pub action: Option<String>,
    /// Call-to-action text (e.g. "Buy Now", "Learn More").
    pub cta: Option<String>,
    /// Human/agent-readable description of the link's purpose.
    pub description: Option<String>,
}
