use axum::extract::Request;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use rmcp::handler::server::common::Extension;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{InitializeRequestParams, InitializeResult, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::StreamableHttpService;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler};
use std::sync::Arc;

use super::models::*;
use crate::services::auth::keys;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::conversions::models::SourceType;
use crate::services::conversions::repo::ConversionsRepository;
use crate::services::links::models::LinkError;
use crate::services::links::models::{
    BulkCreateLinksRequest, BulkCreateLinksResponse, CreateLinkRequest, CreateLinkResponse,
    LinkDetail, ListLinksResponse,
};
use crate::services::links::service::LinksService;

crate::impl_container!(RiftMcp);
pub struct RiftMcp {
    service: Arc<LinksService>,
    secret_keys_repo: Arc<dyn SecretKeysRepository>,
    conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    public_url: String,
    tool_router: ToolRouter<Self>,
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
            tool_router: Self::tool_router(),
        }
    }

    /// Resolve auth from the `McpApiKey` injected by `extract_api_key_header`
    /// into the request extensions. Called per-tool-invocation rather than
    /// once at session init: the streamable-HTTP server runs in stateless
    /// mode (`NeverSessionManager`), so each request gets a fresh `RiftMcp`
    /// instance and there is no session to carry resolved auth state on.
    /// REST middleware does the same per-request lookup (see
    /// `api::auth::middleware::validate_api_key`).
    ///
    /// MCP rejects affiliate-scoped keys here for the same reason as before:
    /// the tools don't yet honor partner scope, and silently granting Full
    /// would let a partner create unattributed links via MCP.
    async fn auth_context(&self, parts: &Parts) -> Result<AuthContext, String> {
        let api_key = parts
            .extensions
            .get::<McpApiKey>()
            .map(|k| k.0.clone())
            .ok_or_else(|| "Missing x-api-key header".to_string())?;

        let key_hash = keys::hash_key(&api_key);
        let key_doc = self
            .secret_keys_repo
            .find_by_hash(&key_hash)
            .await
            .map_err(|e| format!("Key lookup failed: {e}"))?
            .ok_or_else(|| "Invalid or unverified API key".to_string())?;

        if matches!(key_doc.scope, Some(KeyScope::Affiliate { .. })) {
            return Err(
                "MCP does not support partner-scoped credentials. Use a full-tenant rl_live_ key."
                    .to_string(),
            );
        }

        Ok(AuthContext::for_secret_key(
            key_doc.tenant_id,
            key_doc.id,
            Some(&KeyScope::Full),
        ))
    }

    fn webhook_url_for(&self, url_token: &str) -> String {
        format!("{}/w/{}", self.public_url.trim_end_matches('/'), url_token)
    }
}

#[tool_router]
impl RiftMcp {
    #[tool(
        name = "links.create",
        description = "Create a new Rift deep link with platform-specific destinations",
        annotations(
            title = "Create link",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, req, parts), fields(tool = "links.create"))]
    async fn create_link(
        &self,
        Parameters(req): Parameters<CreateLinkRequest>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<CreateLinkResponse>, String> {
        let ctx = self.auth_context(&parts).await?;
        let resp = self
            .service
            .create_link(&ctx, req)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Json(resp))
    }

    #[tool(
        name = "links.bulk_create",
        description = "Atomically create up to 100 Rift deep links sharing one template. Provide either `custom_ids` (caller-supplied slugs) or `count` (auto-generated). Requires a verified custom domain. Returns all created links or an error listing every per-row problem so they can be fixed in one pass.",
        annotations(
            title = "Bulk create links",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, req, parts), fields(tool = "links.bulk_create"))]
    async fn create_links(
        &self,
        Parameters(req): Parameters<BulkCreateLinksRequest>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<BulkCreateLinksResponse>, String> {
        let ctx = self.auth_context(&parts).await?;
        match self.service.create_links_bulk(&ctx, req).await {
            Ok(resp) => Ok(Json(resp)),
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

    #[tool(
        name = "links.get",
        description = "Get details of a Rift deep link by its ID",
        annotations(
            title = "Get link",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, parts), fields(tool = "links.get"))]
    async fn get_link(
        &self,
        Parameters(input): Parameters<GetLinkInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<LinkDetail>, String> {
        let ctx = self.auth_context(&parts).await?;
        let detail = self
            .service
            .get_link(&ctx, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Json(detail))
    }

    #[tool(
        name = "links.list",
        description = "List your Rift deep links with cursor-based pagination",
        annotations(
            title = "List links",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, parts), fields(tool = "links.list"))]
    async fn list_links(
        &self,
        Parameters(input): Parameters<ListLinksInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<ListLinksResponse>, String> {
        let ctx = self.auth_context(&parts).await?;
        let resp = self
            .service
            .list_links(&ctx, input.limit, input.cursor)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Json(resp))
    }

    #[tool(
        name = "links.update",
        description = "Update an existing Rift deep link's destinations or metadata. Omit a field to leave it unchanged; pass `null` to clear a nullable field (e.g. `\"ios_deep_link\": null`).",
        annotations(
            title = "Update link",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, input, parts), fields(tool = "links.update"))]
    async fn update_link(
        &self,
        Parameters(input): Parameters<UpdateLinkInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<LinkDetail>, String> {
        let ctx = self.auth_context(&parts).await?;
        let detail = self
            .service
            .update_link(&ctx, &input.link_id, input.body)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Json(detail))
    }

    #[tool(
        name = "links.delete",
        description = "Delete a Rift deep link permanently",
        annotations(
            title = "Delete link",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, parts), fields(tool = "links.delete"))]
    async fn delete_link(
        &self,
        Parameters(input): Parameters<DeleteLinkInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<DeleteLinkOutput>, String> {
        let ctx = self.auth_context(&parts).await?;
        self.service
            .delete_link(&ctx, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Json(DeleteLinkOutput {
            link_id: input.link_id,
            deleted: true,
        }))
    }

    #[tool(
        name = "sources.create",
        description = "Create a new conversion tracking source. Returns a webhook URL that your backend POSTs conversion events to. v1 supports source_type \"custom\" only — point your own backend at the returned URL. Conversions are attributed to links via the user_id field in the event payload.",
        annotations(
            title = "Create conversion source",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, parts), fields(tool = "sources.create"))]
    async fn create_source(
        &self,
        Parameters(input): Parameters<CreateSourceInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<CreateSourceOutput>, String> {
        let tenant_id = self.auth_context(&parts).await?.tenant_id;
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
        Ok(Json(CreateSourceOutput {
            id: source.id,
            name: source.name,
            source_type: source.source_type,
            webhook_url,
        }))
    }

    #[tool(
        name = "sources.list",
        description = "List all conversion tracking sources for your tenant. Auto-creates a 'default' custom source if none exist, so the first call always returns at least one usable webhook URL.",
        annotations(
            title = "List conversion sources",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    #[tracing::instrument(skip(self, parts), fields(tool = "sources.list"))]
    async fn list_sources(
        &self,
        Parameters(_input): Parameters<ListSourcesInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<ListSourcesOutput>, String> {
        let tenant_id = self.auth_context(&parts).await?.tenant_id;
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

        let summaries: Vec<SourceSummary> = sources
            .into_iter()
            .map(|s| SourceSummary {
                webhook_url: self.webhook_url_for(&s.url_token),
                id: s.id,
                name: s.name,
                source_type: s.source_type,
                created_at: s.created_at.try_to_rfc3339_string().unwrap_or_default(),
            })
            .collect();

        Ok(Json(ListSourcesOutput { sources: summaries }))
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
        // Auth is resolved per-tool-call from the `x-api-key` header (see
        // `auth_context`). The streamable-HTTP server runs in stateless mode
        // (`NeverSessionManager`), so anything we stored on this `RiftMcp`
        // instance during initialize would not survive to the next request
        // anyway. Initialize still validates the key when it's present so
        // bad credentials fail fast at session start — but a missing key is
        // not fatal here, the failure surfaces on the first tool call.
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
            });

        if let Some(api_key) = api_key {
            let key_hash = keys::hash_key(&api_key);
            let key_doc = self
                .secret_keys_repo
                .find_by_hash(&key_hash)
                .await
                .map_err(|e| McpError::internal_error(format!("Key lookup failed: {e}"), None))?
                .ok_or_else(|| McpError::invalid_params("Invalid or unverified API key", None))?;

            if matches!(key_doc.scope, Some(KeyScope::Affiliate { .. })) {
                return Err(McpError::invalid_params(
                    "MCP does not support partner-scoped credentials. Use a full-tenant rl_live_ key.",
                    None,
                ));
            }

            let tenant_id = key_doc.tenant_id;
            tracing::info!(%tenant_id, "MCP session authenticated");
            sentry::configure_scope(|s| {
                s.set_tag("tenant_id", tenant_id.to_string());
                s.set_tag("transport", "mcp");
            });
        }

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
        // Stateless + JSON: our tools are request-response only, so sessions
        // are dead weight and break across multi-instance deployments where
        // initialize and tools/list can hit different machines. `json_response`
        // returns application/json directly instead of SSE, which simple MCP
        // clients parse more reliably. If we ever add streaming tools
        // (progress, subscriptions, sampling), swap NeverSessionManager for a
        // MongoDB-backed SessionManager and flip both flags back.
        Arc::new(NeverSessionManager::default()),
        StreamableHttpServerConfig {
            stateful_mode: false,
            json_response: true,
            ..Default::default()
        },
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
