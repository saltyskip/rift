use mongodb::bson::oid::ObjectId;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{InitializeRequestParams, InitializeResult, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::StreamableHttpService;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler};
use std::sync::{Arc, OnceLock};

use super::tools::*;
use crate::api::auth::keys;
use crate::api::auth::repo::AuthRepository;
use crate::api::links::models::{AgentContext, CreateLinkRequest, UpdateLinkRequest};
use crate::api::links::service::LinksService;

pub struct RiftMcp {
    service: Arc<LinksService>,
    auth_repo: Arc<dyn AuthRepository>,
    tenant_id: OnceLock<ObjectId>,
    tool_router: ToolRouter<Self>,
}

impl RiftMcp {
    pub fn new(service: Arc<LinksService>, auth_repo: Arc<dyn AuthRepository>) -> Self {
        Self {
            service,
            auth_repo,
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
}

#[tool_router]
impl RiftMcp {
    #[tool(description = "Create a new Rift deep link with platform-specific destinations")]
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
            agent_context: input.agent_context.map(|ac| AgentContext {
                action: ac.action,
                cta: ac.cta,
                description: ac.description,
            }),
        };

        let resp = self
            .service
            .create_link(tenant_id, req)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Created link: {}\nURL: {}", resp.link_id, resp.url))
    }

    #[tool(description = "Get details of a Rift deep link by its ID")]
    async fn get_link(
        &self,
        Parameters(input): Parameters<GetLinkInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let detail = self
            .service
            .get_link(&tenant_id, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "List your Rift deep links with cursor-based pagination")]
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
    async fn update_link(
        &self,
        Parameters(input): Parameters<UpdateLinkInput>,
    ) -> Result<String, String> {
        let tenant_id = self.tenant_id()?;
        let req = UpdateLinkRequest {
            ios_deep_link: input.ios_deep_link,
            android_deep_link: input.android_deep_link,
            web_url: input.web_url,
            ios_store_url: input.ios_store_url,
            android_store_url: input.android_store_url,
            metadata: input.metadata,
            agent_context: input.agent_context.map(|ac| AgentContext {
                action: ac.action,
                cta: ac.cta,
                description: ac.description,
            }),
        };

        let detail = self
            .service
            .update_link(&tenant_id, &input.link_id, req)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "Delete a Rift deep link permanently")]
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
}

#[tool_handler]
impl ServerHandler for RiftMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
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
        // Extract api_key from _meta (rmcp moves _meta into context.meta during dispatch)
        let api_key = context
            .meta
            .0
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Missing _meta.api_key in initialize request", None)
            })?;

        // Hash and look up the key
        let key_hash = keys::hash_key(api_key);
        let key_doc = self
            .auth_repo
            .find_key_by_hash(&key_hash)
            .await
            .ok_or_else(|| McpError::invalid_params("Invalid or unverified API key", None))?;

        let tenant_id = key_doc.id.unwrap_or_else(ObjectId::new);
        self.tenant_id
            .set(tenant_id)
            .map_err(|_| McpError::internal_error("Session already authenticated", None))?;

        tracing::info!(%tenant_id, "MCP session authenticated");

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
    auth_repo: Arc<dyn AuthRepository>,
) -> axum::Router {
    let service = StreamableHttpService::new(
        move || Ok(RiftMcp::new(links_service.clone(), auth_repo.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    axum::Router::new().nest_service("/mcp", service)
}
