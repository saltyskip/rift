use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::{oid::ObjectId, DateTime};
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::models::TenantId;
use crate::app::AppState;
use crate::services::webhooks::models::*;

#[utoipa::path(
    post,
    path = "/v1/webhooks",
    tag = "Webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created", body = CreateWebhookResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateWebhookRequest>,
) -> Response {
    let Some(svc) = &state.webhooks_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    if req.events.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "At least one event type is required", "code": "empty_events" })),
        )
            .into_response();
    }

    if let Err(e) = validate_webhook_url(&req.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e, "code": "invalid_url" })),
        )
            .into_response();
    }

    let secret = generate_secret();
    let now = DateTime::now();
    let id = ObjectId::new();

    match svc
        .create_webhook(
            tenant.0,
            id,
            req.url.clone(),
            secret.clone(),
            req.events.clone(),
            now,
        )
        .await
    {
        Ok(_) => (
            StatusCode::CREATED,
            Json(CreateWebhookResponse {
                id: id.to_hex(),
                url: req.url,
                events: req.events,
                secret,
                created_at: now.try_to_rfc3339_string().unwrap_or_default(),
            }),
        )
            .into_response(),
        Err(crate::services::webhooks::service::WebhookError::QuotaExceeded(q)) => {
            crate::api::billing::quota_response::to_response(q)
        }
        Err(crate::services::webhooks::service::WebhookError::Internal(e)) => {
            tracing::error!("Failed to create webhook: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/webhooks",
    tag = "Webhooks",
    responses(
        (status = 200, description = "List of webhooks", body = ListWebhooksResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(repo) = &state.webhooks_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match repo.list_by_tenant(&tenant.0).await {
        Ok(webhooks) => {
            let details: Vec<WebhookDetail> = webhooks
                .into_iter()
                .map(|w| WebhookDetail {
                    id: w.id.to_hex(),
                    url: w.url,
                    events: w.events,
                    active: w.active,
                    created_at: w.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(ListWebhooksResponse { webhooks: details }).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list webhooks: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/v1/webhooks/{webhook_id}",
    tag = "Webhooks",
    params(("webhook_id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 204, description = "Webhook deleted"),
        (status = 404, description = "Webhook not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(webhook_id): Path<String>,
) -> Response {
    let Some(repo) = &state.webhooks_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&webhook_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid webhook ID", "code": "invalid_id" })),
        )
            .into_response();
    };

    match repo.delete_webhook(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Webhook not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete webhook: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    patch,
    path = "/v1/webhooks/{webhook_id}",
    tag = "Webhooks",
    params(("webhook_id" = String, Path, description = "Webhook ID")),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Webhook updated", body = WebhookDetail),
        (status = 404, description = "Webhook not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn patch_webhook(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(webhook_id): Path<String>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Response {
    let Some(repo) = &state.webhooks_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&webhook_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid webhook ID", "code": "invalid_id" })),
        )
            .into_response();
    };

    match repo.set_active(&tenant.0, &oid, req.active).await {
        Ok(true) => {
            // Fetch updated webhook to return.
            let webhooks = repo.list_by_tenant(&tenant.0).await.unwrap_or_default();
            match webhooks.iter().find(|w| w.id == oid) {
                Some(w) => Json(json!({
                    "id": w.id.to_hex(),
                    "url": w.url,
                    "events": w.events,
                    "active": w.active,
                    "created_at": w.created_at.try_to_rfc3339_string().unwrap_or_default(),
                }))
                .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Webhook not found", "code": "not_found" })),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Webhook not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update webhook: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

fn validate_webhook_url(raw: &str) -> Result<(), String> {
    let parsed = url::Url::parse(raw).map_err(|_| "Invalid URL".to_string())?;
    if parsed.scheme() != "https" {
        return Err("Webhook URL must use HTTPS".to_string());
    }
    if parsed.host_str().is_none() {
        return Err("Webhook URL must have a host".to_string());
    }
    Ok(())
}

fn generate_secret() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}
