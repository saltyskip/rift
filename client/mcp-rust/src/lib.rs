//! `riftl-mcp` — drop-in instrumentation for [`rmcp`]-based MCP servers.
//!
//! Wrap your `ServerHandler` once and every `tools/call` is captured and shipped
//! to Rift's ingest endpoint — no per-tool code. This is the **sensor** half of
//! the Rift Agent Layer; attribution + the agent→human handoff build on top
//! server-side.
//!
//! ```ignore
//! use riftl_mcp::{InstrumentExt, RiftConfig};
//!
//! let server = MyMcpServer::new(deps).instrument(RiftConfig {
//!     ingest_url: "https://api.riftl.ink/v1/agents/actions".into(),
//!     api_key: std::env::var("RIFT_KEY").unwrap(),
//! });
//! // serve `server` exactly as before — it's a ServerHandler.
//! ```
//!
//! ## Design contract
//! - **Auto** — hooks the single `call_tool` chokepoint, so all tools (present
//!   and future) are captured. Other `ServerHandler` methods delegate unchanged.
//! - **Fail-open** — capture runs off the hot path (`tokio::spawn`); a network
//!   error or a down Rift never blocks or fails the tool call.
//! - **Decoupled** — the default [`HttpEmitter`] POSTs to Rift, but embedders
//!   can inject any [`ActionEmitter`] (e.g. an in-process sink).

use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, InitializeRequestParams, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};

// ── Public data types ──

/// Connection settings for the default [`HttpEmitter`].
#[derive(Debug, Clone)]
pub struct RiftConfig {
    /// Full ingest URL, e.g. `https://api.riftl.ink/v1/agents/actions`.
    pub ingest_url: String,
    /// Tenant secret key (`rl_live_…`), sent as a Bearer token.
    pub api_key: String,
}

/// One captured tool call. Serializes to the body of `POST /v1/agents/actions`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentAction {
    /// Tool name that was invoked.
    pub tool: String,
    /// Self-reported calling agent (`clientInfo.name`), when announced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_platform: Option<String>,
    /// Tool arguments, captured verbatim as intent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<serde_json::Value>,
    /// `ok` | `error` — outcome of the underlying tool call.
    pub status: String,
    /// Wall-clock duration of the tool call in milliseconds.
    pub latency_ms: u32,
    /// Reserved for the handoff rail (v1); always `false` today.
    pub mint_journey_token: bool,
}

// ── Emitter ──

/// Sink for captured actions. The crate ships [`HttpEmitter`]; embedders can
/// provide an in-process implementation. Implementations **must not block** —
/// `emit` is called inline in the tool-call path and returns immediately.
pub trait ActionEmitter: Send + Sync + 'static {
    fn emit(&self, action: AgentAction);
}

/// Default emitter: POST each action to Rift's ingest endpoint. Spawns the
/// request so the tool call never waits and never fails on a network error.
pub struct HttpEmitter {
    client: reqwest::Client,
    url: String,
    api_key: String,
}

impl HttpEmitter {
    pub fn new(config: RiftConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: config.ingest_url,
            api_key: config.api_key,
        }
    }
}

impl ActionEmitter for HttpEmitter {
    fn emit(&self, action: AgentAction) {
        let client = self.client.clone();
        let url = self.url.clone();
        let api_key = self.api_key.clone();
        // Fire-and-forget: never blocks the tool call, never propagates errors.
        tokio::spawn(async move {
            if let Err(e) = client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&action)
                .send()
                .await
            {
                tracing::debug!(error = %e, "riftl-mcp: action emit failed (ignored)");
            }
        });
    }
}

// ── The wrapper ──

/// A `ServerHandler` decorator that records every tool call. Construct via
/// [`InstrumentExt::instrument`] rather than directly in most cases.
pub struct Instrumented<H> {
    inner: H,
    emitter: Arc<dyn ActionEmitter>,
}

impl<H> Instrumented<H> {
    /// Wrap with an explicit emitter (e.g. an in-process sink).
    pub fn new(inner: H, emitter: Arc<dyn ActionEmitter>) -> Self {
        Self { inner, emitter }
    }

    /// Wrap with the default HTTP emitter built from `config`.
    pub fn with_config(inner: H, config: RiftConfig) -> Self {
        Self::new(inner, Arc::new(HttpEmitter::new(config)))
    }
}

impl<H: ServerHandler> ServerHandler for Instrumented<H> {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool = request.name.to_string();
        // Self-reported client identity; absent in stateless-HTTP transports.
        let agent_platform = context
            .peer
            .peer_info()
            .map(|info| info.client_info.name.clone());
        let intent = request.arguments.clone().map(serde_json::Value::Object);

        let started = Instant::now();
        let result = self.inner.call_tool(request, context).await;
        let latency_ms = started.elapsed().as_millis().min(u32::MAX as u128) as u32;

        self.emitter.emit(AgentAction {
            tool,
            agent_platform,
            intent,
            status: if result.is_ok() { "ok" } else { "error" }.to_string(),
            latency_ms,
            mint_journey_token: false,
        });

        result
    }

    // ── Delegate the rest so the inner handler's behavior is preserved ──
    // (`call_tool` is the only chokepoint we instrument; `list_tools`/`get_tool`
    // keep the macro-generated tool table visible, `get_info`/`initialize` keep
    // the inner server's identity + handshake intact.)

    fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        self.inner.list_tools(request, context)
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.inner.get_tool(name)
    }

    fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        self.inner.initialize(request, context)
    }

    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }
}

// ── Ergonomic one-liner ──

/// Blanket extension trait giving every `ServerHandler` an `.instrument(...)`
/// method — the one-line drop-in.
pub trait InstrumentExt: ServerHandler + Sized {
    /// Wrap with the default HTTP emitter.
    fn instrument(self, config: RiftConfig) -> Instrumented<Self> {
        Instrumented::with_config(self, config)
    }

    /// Wrap with a custom emitter.
    fn instrument_with(self, emitter: Arc<dyn ActionEmitter>) -> Instrumented<Self> {
        Instrumented::new(self, emitter)
    }
}

impl<H: ServerHandler> InstrumentExt for H {}
