pub mod tools;

use mongodb::bson::oid::ObjectId;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ServerInfo;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use std::sync::Arc;

use crate::api::links::models::{AgentContext, CreateLinkRequest, UpdateLinkRequest};
use crate::api::links::service::LinksService;
use tools::*;

pub struct RiftMcp {
    service: Arc<LinksService>,
    tenant_id: ObjectId,
    tool_router: ToolRouter<Self>,
}

impl RiftMcp {
    pub fn new(service: Arc<LinksService>, tenant_id: ObjectId) -> Self {
        Self {
            service,
            tenant_id,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl RiftMcp {
    #[tool(description = "Create a new Rift deep link with platform-specific destinations")]
    async fn create_link(
        &self,
        Parameters(input): Parameters<CreateLinkInput>,
    ) -> Result<String, String> {
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
            .create_link(self.tenant_id, req)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Created link: {}\nURL: {}", resp.link_id, resp.url))
    }

    #[tool(description = "Get details of a Rift deep link by its ID")]
    async fn get_link(
        &self,
        Parameters(input): Parameters<GetLinkInput>,
    ) -> Result<String, String> {
        let detail = self
            .service
            .get_link(&self.tenant_id, &input.link_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "List your Rift deep links with cursor-based pagination")]
    async fn list_links(
        &self,
        Parameters(input): Parameters<ListLinksInput>,
    ) -> Result<String, String> {
        let resp = self
            .service
            .list_links(&self.tenant_id, input.limit, input.cursor)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())
    }

    #[tool(description = "Update an existing Rift deep link's destinations or metadata")]
    async fn update_link(
        &self,
        Parameters(input): Parameters<UpdateLinkInput>,
    ) -> Result<String, String> {
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
            .update_link(&self.tenant_id, &input.link_id, req)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }

    #[tool(description = "Delete a Rift deep link permanently")]
    async fn delete_link(
        &self,
        Parameters(input): Parameters<DeleteLinkInput>,
    ) -> Result<String, String> {
        self.service
            .delete_link(&self.tenant_id, &input.link_id)
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
}
