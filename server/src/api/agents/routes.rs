use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::models::TenantId;
use crate::app::AppState;
use crate::services::agents::models::{AgentError, RecordActionRequest};

// ── POST /v1/agents/actions — Record one instrumented MCP tool call ──

#[utoipa::path(
    post,
    path = "/v1/agents/actions",
    tag = "Agents",
    request_body = RecordActionRequest,
    responses(
        (status = 200, description = "Action recorded", body = crate::services::agents::models::RecordActionResponse),
        (status = 402, description = "Quota exceeded", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn record_action(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<RecordActionRequest>,
) -> Response {
    let Some(service) = &state.agents_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match service.record_action(&tenant, req).await {
        Ok(resp) => Json(resp).into_response(),
        Err(AgentError::QuotaExceeded(qe)) => crate::api::billing::quota_response::to_response(qe),
        Err(AgentError::Storage(e)) => {
            tracing::error!(error = %e, "Failed to record agent action");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}
