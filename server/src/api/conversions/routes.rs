use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use super::models::SdkConversionRequest;
use crate::api::auth::models::TenantId;
use crate::app::AppState;
use crate::services::conversions::models::{
    CreateSourceRequest, CreateSourceResponse, ListSourcesResponse, Source, SourceDetail,
};
use crate::services::conversions::parsers;

// ── POST /v1/sources — Create a source ──

#[utoipa::path(
    post,
    path = "/v1/sources",
    tag = "Conversions",
    request_body = CreateSourceRequest,
    responses(
        (status = 201, description = "Source created", body = CreateSourceResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 409, description = "Source name already exists", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_source(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateSourceRequest>,
) -> Response {
    let Some(repo) = &state.conversions_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let name = req.name.trim().to_string();
    if name.is_empty() || name.len() > 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Name must be 1-64 characters", "code": "invalid_name" })),
        )
            .into_response();
    }

    match repo.create_source(tenant.0, name, req.source_type).await {
        Ok(source) => {
            let resp = CreateSourceResponse {
                id: source.id.to_hex(),
                name: source.name.clone(),
                source_type: source.source_type.clone(),
                webhook_url: webhook_url_for(&state, &source.url_token),
                created_at: source
                    .created_at
                    .try_to_rfc3339_string()
                    .unwrap_or_default(),
            };
            (
                StatusCode::CREATED,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
        Err(e) if e.contains("E11000") => (
            StatusCode::CONFLICT,
            Json(
                json!({ "error": "A source with that name already exists", "code": "name_taken" }),
            ),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to create source");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── GET /v1/sources — List sources (auto-provisions default) ──

#[utoipa::path(
    get,
    path = "/v1/sources",
    tag = "Conversions",
    responses(
        (status = 200, description = "List of sources", body = ListSourcesResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_sources(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(repo) = &state.conversions_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let mut sources = match repo.list_sources(&tenant.0).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to list sources");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    // Auto-provision a default custom source if the tenant has none. This is the
    // zero-ceremony dev flow: first GET returns a usable webhook URL immediately.
    if sources.is_empty() {
        match repo.get_or_create_default_custom_source(tenant.0).await {
            Ok(source) => sources.push(source),
            Err(e) => {
                tracing::error!(error = %e, "Failed to auto-provision default source");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Internal error", "code": "db_error" })),
                )
                    .into_response();
            }
        }
    }

    let details: Vec<SourceDetail> = sources.iter().map(|s| to_detail(&state, s)).collect();
    Json(ListSourcesResponse { sources: details }).into_response()
}

// ── GET /v1/sources/{id} — Get one source ──

#[utoipa::path(
    get,
    path = "/v1/sources/{id}",
    tag = "Conversions",
    params(("id" = String, Path, description = "Source ID")),
    responses(
        (status = 200, description = "Source detail", body = SourceDetail),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_source(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(id): Path<String>,
) -> Response {
    let Some(repo) = &state.conversions_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Source not found", "code": "not_found" })),
        )
            .into_response();
    };

    match repo.find_source_by_id(&tenant.0, &oid).await {
        Ok(Some(source)) => Json(to_detail(&state, &source)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Source not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to find source");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── DELETE /v1/sources/{id} — Delete a source ──

#[utoipa::path(
    delete,
    path = "/v1/sources/{id}",
    tag = "Conversions",
    params(("id" = String, Path, description = "Source ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_source(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(id): Path<String>,
) -> Response {
    let Some(repo) = &state.conversions_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Source not found", "code": "not_found" })),
        )
            .into_response();
    };

    match repo.delete_source(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Source not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete source");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── POST /w/{token} — Public webhook receiver ──
//
// Deliberately NOT annotated with `#[utoipa::path]`. The body is raw bytes (any
// parser-specific shape), and we don't want it in the OpenAPI schema with a
// generated body type. It's documented in the human-readable docs instead.

#[tracing::instrument(skip(state, body))]
pub async fn receive_webhook(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    body: Bytes,
) -> Response {
    let Some(repo) = &state.conversions_repo else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(service) = &state.conversions_service else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let source = match repo.find_source_by_token(&token).await {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Source lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let parser = parsers::parser_for(source.source_type.clone());
    let parsed = match parser.parse(&body, &source) {
        Ok(events) => events,
        Err(e) => {
            tracing::warn!(
                source_id = %source.id,
                error = ?e,
                "Parser rejected payload",
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string(), "code": "parse_error" })),
            )
                .into_response();
        }
    };

    let result = service.ingest_parsed(&source, parsed).await;

    Json(json!({
        "accepted": result.accepted,
        "deduped": result.deduped,
        "unattributed": result.unattributed,
        "failed": result.failed,
    }))
    .into_response()
}

// ── POST /v1/attribution/convert — SDK-authenticated conversion tracking ──
//
// Same pipeline as the webhook receiver but authenticated by publishable key
// instead of an opaque URL token. The mobile SDK uses this so it doesn't need
// a separate webhook URL in the binary.

#[utoipa::path(
    post,
    path = "/v1/attribution/convert",
    tag = "Conversions",
    request_body = SdkConversionRequest,
    responses(
        (status = 200, description = "Event processed"),
        (status = 400, description = "Invalid payload", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn sdk_track_conversion(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<SdkConversionRequest>,
) -> Response {
    let Some(service) = &state.conversions_service else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    if req.user_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "user_id is required", "code": "bad_request" })),
        )
            .into_response();
    }

    if req.conversion_type.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "type is required", "code": "bad_request" })),
        )
            .into_response();
    }

    let parsed = vec![crate::services::conversions::models::ParsedConversion {
        user_id: Some(req.user_id),
        conversion_type: req.conversion_type,
        idempotency_key: req.idempotency_key,
        metadata: req
            .metadata
            .and_then(|v| mongodb::bson::to_bson(&v).ok())
            .and_then(|b| b.as_document().cloned()),
        occurred_at: None,
    }];

    let result = service.ingest_sdk_event(tenant.0, parsed).await;

    Json(json!({
        "accepted": result.accepted,
        "deduped": result.deduped,
        "unattributed": result.unattributed,
        "failed": result.failed,
    }))
    .into_response()
}

// ── Helpers ──

fn webhook_url_for(state: &AppState, url_token: &str) -> String {
    format!(
        "{}/w/{}",
        state.config.public_url.trim_end_matches('/'),
        url_token
    )
}

fn to_detail(state: &AppState, source: &Source) -> SourceDetail {
    SourceDetail {
        id: source.id.to_hex(),
        name: source.name.clone(),
        source_type: source.source_type.clone(),
        webhook_url: webhook_url_for(state, &source.url_token),
        created_at: source
            .created_at
            .try_to_rfc3339_string()
            .unwrap_or_default(),
    }
}
