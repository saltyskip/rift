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

use super::tools::*;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::conversions::models::SourceType;
use crate::services::conversions::repo::ConversionsRepository;
use crate::services::links::models::LinkError;
use crate::services::links::models::{
    AgentContext, BulkCreateLinksRequest, BulkLinkTemplate, CreateLinkRequest, UpdateLinkRequest,
};
use crate::services::links::service::LinksService;

crate::impl_container!(RiftMcp);
pub struct RiftMcp {
    service: Arc<LinksService>,
    secret_keys_repo: Arc<dyn SecretKeysRepository>,
    conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    public_url: String,
    tenant_id: OnceLock<ObjectId>,
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
            tenant_id: OnceLock::new(),
            tool_router: Self::tool_router(),
        }
    }

    fn tenant_id(&self) -> Result<ObjectId, String> {
        self.tenant_id
            .get()
            .copied()
            .ok_or_else(|| "Not authenticated".to_string())
    }

    fn webhook_url_for(&self, url_token: &str) -> String {
        format!("{}/w/{}", self.public_url.trim_end_matches('/'), url_token)
    }
}

#[tool_router]
impl RiftMcp {
    #[tool(description = "Create a new Rift deep link with platform-specific destinations")]
    #[tracing::instrument(skip(self, input), fields(tool = "create_link"))]
    async fn create_link(
        &self,
        Parameters(input): Parameters<CreateLinkInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let req = CreateLinkRequest {
            custom_id: input.custom_id,
            ios_deep_link: input.ios_deep_link,
            android_deep_link: input.android_deep_link,
            web_url: input.web_url,
            ios_store_url: input.ios_store_url,
            android_store_url: input.android_store_url,
            metadata: input.metadata,
            affiliate_id: None,
            agent_context: input.agent_context.map(|ac| AgentContext {
                action: ac.action,
                cta: ac.cta,
                description: ac.description,
            }),
            social_preview: None,
        };

        // MCP tool sessions authenticate as the tenant; treat as full scope.
        // FUTURE: when MCP grows partner support, source the scope from the
        // session's credential and pass it through here.
        let resp = self
            .service
            .create_link(tenant_id, Some(&KeyScope::Full), req)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Created link: {}\nURL: {}", resp.link_id, resp.url))
    }

    #[tool(
        description = "Atomically create up to 100 Rift deep links sharing one template. Provide either `custom_ids` (caller-supplied slugs) or `count` (auto-generated). Requires a verified custom domain. Returns all created links or an error listing every per-row problem so they can be fixed in one pass."
    )]
    #[tracing::instrument(skip(self, input), fields(tool = "create_links"))]
    async fn create_links(
        &self,
        Parameters(input): Parameters<CreateLinksInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let req = BulkCreateLinksRequest {
            template: BulkLinkTemplate {
                ios_deep_link: input.template.ios_deep_link,
                android_deep_link: input.template.android_deep_link,
                web_url: input.template.web_url,
                ios_store_url: input.template.ios_store_url,
                android_store_url: input.template.android_store_url,
                metadata: input.template.metadata,
                affiliate_id: None,
                agent_context: input.template.agent_context.map(|ac| AgentContext {
                    action: ac.action,
                    cta: ac.cta,
                    description: ac.description,
                }),
                social_preview: None,
            },
            custom_ids: input.custom_ids,
            count: input.count,
        };

        match self
            .service
            .create_links_bulk(tenant_id, Some(&KeyScope::Full), req)
            .await
        {
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
        let tenant_id = self.tenant_id()?;
        // MCP rejects affiliate-scoped keys at init (see initialize), so the
        // caller is always Full-or-grandfathered. Passing None means "no
        // scope filter" — equivalent to Full for read.
        let detail = self
            .service
            .get_link(&tenant_id, None, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "List your Rift deep links with cursor-based pagination")]
    #[tracing::instrument(skip(self), fields(tool = "list_links"))]
    async fn list_links(
        &self,
        Parameters(input): Parameters<ListLinksInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let resp = self
            .service
            .list_links(&tenant_id, input.limit, input.cursor)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())
    }

    #[tool(description = "Update an existing Rift deep link's destinations or metadata")]
    #[tracing::instrument(skip(self, input), fields(tool = "update_link"))]
    async fn update_link(
        &self,
        Parameters(input): Parameters<UpdateLinkInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let ios_deep_link = if input.clear_ios_deep_link {
            Some(None) // unset
        } else {
            input.ios_deep_link.map(Some) // set or leave unchanged
        };
        let android_deep_link = if input.clear_android_deep_link {
            Some(None)
        } else {
            input.android_deep_link.map(Some)
        };
        let req = UpdateLinkRequest {
            ios_deep_link,
            android_deep_link,
            web_url: input.web_url,
            ios_store_url: input.ios_store_url,
            android_store_url: input.android_store_url,
            metadata: input.metadata,
            agent_context: input.agent_context.map(|ac| AgentContext {
                action: ac.action,
                cta: ac.cta,
                description: ac.description,
            }),
            social_preview: None,
        };

        let detail = self
            .service
            .update_link(&tenant_id, &input.link_id, req)
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
        let tenant_id = self.tenant_id()?;
        self.service
            .delete_link(&tenant_id, &input.link_id)
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
        self.tenant_id
            .set(tenant_id)
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
