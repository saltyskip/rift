use async_trait::async_trait;
use mongodb::options::{TimeseriesGranularity, TimeseriesOptions};
use mongodb::{Collection, Database};

use super::models::AgentActionEvent;

// ── Trait ──

#[async_trait]
pub trait AgentActionsRepository: Send + Sync {
    /// Append one agent-action event to the time series collection.
    async fn insert_action(&self, event: AgentActionEvent) -> Result<(), String>;
}

// ── Repository ──

crate::impl_container!(AgentActionsRepo);
#[derive(Clone)]
pub struct AgentActionsRepo {
    events: Collection<AgentActionEvent>,
}

impl AgentActionsRepo {
    pub async fn new(database: &Database) -> Self {
        // agent_action_events is a time series collection — same pattern as
        // click_events / conversion_events (append-only, bucketed by `meta`).
        let ts_opts = TimeseriesOptions::builder()
            .time_field("occurred_at".to_string())
            .meta_field(Some("meta".to_string()))
            .granularity(Some(TimeseriesGranularity::Minutes))
            .build();
        if let Err(e) = database
            .create_collection("agent_action_events")
            .timeseries(ts_opts)
            .await
        {
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("48") {
                tracing::error!("Failed to create agent_action_events time series collection: {e}");
            }
        }
        let events = database.collection::<AgentActionEvent>("agent_action_events");

        // Per-tier retention — same partial TTL pattern as click_events.
        crate::services::billing::retention::ensure_retention_ttl_indexes(
            &events,
            "occurred_at",
            "meta",
        )
        .await;

        AgentActionsRepo { events }
    }
}

#[async_trait]
impl AgentActionsRepository for AgentActionsRepo {
    async fn insert_action(&self, event: AgentActionEvent) -> Result<(), String> {
        self.events
            .insert_one(&event)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
