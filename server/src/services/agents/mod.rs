//! Agent Layer — measurement + (future) controlled-handoff for instrumented MCP
//! servers. v0 ships the capture primitive (`AgentActionEvent`) and the
//! `POST /v1/agents/actions` ingest. See `docs/agent-layer-spec.md`.

pub mod models;
pub mod repo;
pub mod service;
