use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use mongodb::bson::oid::ObjectId;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{InitializeRequestParams, InitializeResult, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::StreamableHttpService;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler};
use std::sync::{Arc, OnceLock};

use super::models::*;
use crate::services::auth::keys;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::conversions::models::SourceType;
use crate::services::conversions::repo::ConversionsRepository;
use crate::services::links::models::LinkError;
use crate::services::links::models::{BulkCreateLinksRequest, CreateLinkRequest};
use crate::services::links::service::LinksService;

crate::impl_container!(RiftMcp);
pub struct RiftMcp {
    service: Arc<LinksService>,
    secret_keys_repo: Arc<dyn SecretKeysRepository>,
    conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    public_url: String,
    /// `(tenant_id, key_id)` resolved during `initialize`. Used to build
    /// `AuthContext` for each tool call.
    session: OnceLock<McpSession>,
    tool_router: ToolRouter<Self>,
}

#[derive(Clone, Copy)]
struct McpSession {
    tenant_id: ObjectId,
    key_id: ObjectId,
}

impl RiftMcp {
    pub fn new(
        service: Arc<LinksService>,
        secret_keys_repo: Arc<dyn SecretKeysRepository>,
        conversions_repo: Option<Arc<dyn ConversionsRepository>>,
        public_url: String,
    ) -> Self {
        Self {
            service,
            secret_keys_repo,
            conversions_repo,
            public_url,
            session: OnceLock::new(),
            tool_router: Self::tool_router(),
        }
    }

    fn tenant_id(&self) -> Result<ObjectId, String> {
        self.session
            .get()
            .map(|s| s.tenant_id)
            .ok_or_else(|| "Not authenticated".to_string())
    }

    /// Build an `AuthContext` for this MCP session. MCP rejects affiliate-
    /// scoped keys at init, so we always synthesize a Full-tenant context.
    fn auth_context(&self) -> Result<AuthContext, String> {
        let s = self
            .session
            .get()
            .ok_or_else(|| "Not authenticated".to_string())?;
        Ok(AuthContext::for_secret_key(
            s.tenant_id,
            s.key_id,
            Some(&KeyScope::Full),
        ))
    }

    fn webhook_url_for(&self, url_token: &str) -> String {
        format!("{}/w/{}", self.public_url.trim_end_matches('/'), url_token)
    }
}

#[tool_router]
impl RiftMcp {
    #[tool(description = "Create a new Rift deep link with platform-specific destinations")]
    #[tracing::instrument(skip(self, req), fields(tool = "create_link"))]
    async fn create_link(
        &self,
        Parameters(req): Parameters<CreateLinkRequest>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        let resp = self
            .service
            .create_link(&ctx, req)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Created link: {}\nURL: {}", resp.link_id, resp.url))
    }

    #[tool(
        description = "Atomically create up to 100 Rift deep links sharing one template. Provide either `custom_ids` (caller-supplied slugs) or `count` (auto-generated). Requires a verified custom domain. Returns all created links or an error listing every per-row problem so they can be fixed in one pass."
    )]
    #[tracing::instrument(skip(self, req), fields(tool = "create_links"))]
    async fn create_links(
        &self,
        Parameters(req): Parameters<BulkCreateLinksRequest>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        match self.service.create_links_bulk(&ctx, req).await {
            Ok(resp) => serde_json::to_string_pretty(&resp).map_err(|e| e.to_string()),
            Err(LinkError::BatchValidationFailed(errors)) => {
                // Render the per-row failures as a bullet list so the model
                // can self-correct in the next call.
                let lines: Vec<String> = errors
                    .iter()
                    .map(|e| format!("  - index {}: {} ({})", e.index, e.message, e.code,))
                    .collect();
                Err(format!(
                    "{} item(s) failed validation:\n{}",
                    errors.len(),
                    lines.join("\n")
                ))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(description = "Get details of a Rift deep link by its ID")]
    #[tracing::instrument(skip(self), fields(tool = "get_link"))]
    async fn get_link(
        &self,
        Parameters(input): Parameters<GetLinkInput>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        let detail = self
            .service
            .get_link(&ctx, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Get performance stats for a Rift deep link: click_count (landing-page visits), install_count (first-launch installs attributed to this link), identify_count (installs that bound a user_id via /v1/lifecycle/identify), convert_count (sum of conversion events), and a per-type conversions breakdown."
    )]
    #[tracing::instrument(skip(self), fields(tool = "get_link_stats"))]
    async fn get_link_stats(
        &self,
        Parameters(input): Parameters<GetLinkStatsInput>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        let stats = self
            .service
            .get_link_stats(&ctx, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())
    }

    #[tool(description = "List your Rift deep links with cursor-based pagination")]
    #[tracing::instrument(skip(self), fields(tool = "list_links"))]
    async fn list_links(
        &self,
        Parameters(input): Parameters<ListLinksInput>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        let resp = self
            .service
            .list_links(&ctx, input.limit, input.cursor)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Update an existing Rift deep link's destinations or metadata. Omit a field to leave it unchanged; pass `null` to clear a nullable field (e.g. `\"ios_deep_link\": null`)."
    )]
    #[tracing::instrument(skip(self, input), fields(tool = "update_link"))]
    async fn update_link(
        &self,
        Parameters(input): Parameters<UpdateLinkInput>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        let detail = self
            .service
            .update_link(&ctx, &input.link_id, input.body)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "Delete a Rift deep link permanently")]
    #[tracing::instrument(skip(self), fields(tool = "delete_link"))]
    async fn delete_link(
        &self,
        Parameters(input): Parameters<DeleteLinkInput>,
    ) -> Result<String, String> {
        let ctx = self.auth_context()?;
        self.service
            .delete_link(&ctx, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Deleted link: {}", input.link_id))
    }

    #[tool(
        description = "Create a new conversion tracking source. Returns a webhook URL that your backend POSTs conversion events to. v1 supports source_type \"custom\" only — point your own backend at the returned URL. Conversions are attributed to links via the user_id field in the event payload."
    )]
    #[tracing::instrument(skip(self), fields(tool = "create_source"))]
    async fn create_source(
        &self,
        Parameters(input): Parameters<CreateSourceInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let repo = self
            .conversions_repo
            .as_ref()
            .ok_or_else(|| "Conversions not configured".to_string())?;

        let name = input.name.trim().to_string();
        if name.is_empty() || name.len() > 64 {
            return Err("Name must be 1-64 characters".to_string());
        }

        let source_type = match input.source_type.to_lowercase().as_str() {
            "custom" => SourceType::Custom,
            other => return Err(format!("Unsupported source_type: {other}")),
        };

        let source = repo
            .create_source(tenant_id, name, source_type)
            .await
            .map_err(|e| e.to_string())?;

        let webhook_url = self.webhook_url_for(&source.url_token);
        Ok(format!(
            "Created source: {}\nID: {}\nWebhook URL: {}",
            source.name,
            source.id.to_hex(),
            webhook_url
        ))
    }

    #[tool(
        description = "List all conversion tracking sources for your tenant. Auto-creates a 'default' custom source if none exist, so the first call always returns at least one usable webhook URL."
    )]
    #[tracing::instrument(skip(self), fields(tool = "list_sources"))]
    async fn list_sources(
        &self,
        Parameters(_input): Parameters<ListSourcesInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let repo = self
            .conversions_repo
            .as_ref()
            .ok_or_else(|| "Conversions not configured".to_string())?;

        let mut sources = repo
            .list_sources(&tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        if sources.is_empty() {
            let default = repo
                .get_or_create_default_custom_source(tenant_id)
                .await
                .map_err(|e| e.to_string())?;
            sources.push(default);
        }

        let summary: Vec<_> = sources
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_hex(),
                    "name": s.name,
                    "source_type": s.source_type,
                    "webhook_url": self.webhook_url_for(&s.url_token),
                    "created_at": s.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({ "sources": summary }))
            .map_err(|e| e.to_string())
    }
}

#[tool_handler]
impl ServerHandler for RiftMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "rift-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions("Rift MCP server — manage deep links for humans and agents.")
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Try _meta.api_key first, then fall back to x-api-key HTTP header
        let api_key = context
            .meta
            .0
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                context
                    .extensions
                    .get::<axum::http::request::Parts>()
                    .and_then(|parts| parts.extensions.get::<McpApiKey>())
                    .map(|k| k.0.clone())
            })
            .ok_or_else(|| {
                McpError::invalid_params(
                    "Missing API key: provide _meta.api_key or x-api-key header",
                    None,
                )
            })?;

        // Hash and look up the key
        let key_hash = keys::hash_key(&api_key);

        let key_doc = self
            .secret_keys_repo
            .find_by_hash(&key_hash)
            .await
            .map_err(|e| McpError::internal_error(format!("Key lookup failed: {e}"), None))?
            .ok_or_else(|| McpError::invalid_params("Invalid or unverified API key", None))?;

        // Reject affiliate-scoped credentials. The MCP tools (`create_link`,
        // `get_link`, etc.) don't yet honor partner scope, and silently
        // granting Full would let a partner create unattributed links via
        // MCP. When MCP grows partner support, source the scope from the
        // session here and pass it through to each tool's service call.
        if matches!(key_doc.scope, Some(KeyScope::Affiliate { .. })) {
            return Err(McpError::invalid_params(
                "MCP does not support partner-scoped credentials. Use a full-tenant rl_live_ key.",
                None,
            ));
        }

        let tenant_id = key_doc.tenant_id;
        self.session
            .set(McpSession {
                tenant_id,
                key_id: key_doc.id,
            })
            .map_err(|_| McpError::internal_error("Session already authenticated", None))?;

        tracing::info!(%tenant_id, "MCP session authenticated");
        sentry::configure_scope(|s| {
            s.set_tag("tenant_id", tenant_id.to_string());
            s.set_tag("transport", "mcp");
        });

        // Default behavior: store peer info and return server info
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }
}

/// Build an Axum router for the MCP server using streamable HTTP transport.
pub fn mcp_router(
    links_service: Arc<LinksService>,
    secret_keys_repo: Arc<dyn SecretKeysRepository>,
    conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    public_url: String,
) -> axum::Router {
    let service = StreamableHttpService::new(
        move || {
            Ok(RiftMcp::new(
                links_service.clone(),
                secret_keys_repo.clone(),
                conversions_repo.clone(),
                public_url.clone(),
            ))
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn(extract_api_key_header))
}

// ── Helpers ──

/// API key extracted from HTTP `x-api-key` header, injected by middleware.
#[derive(Debug, Clone)]
struct McpApiKey(String);

async fn extract_api_key_header(mut req: Request, next: Next) -> Response {
    let key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if let Some(key) = key {
        req.extensions_mut().insert(McpApiKey(key));
    }
    next.run(req).await
}
