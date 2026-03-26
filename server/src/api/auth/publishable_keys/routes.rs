use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde_json::json;
use std::sync::Arc;

use super::models::*;
use crate::api::auth::keys;
use crate::api::auth::middleware::TenantId;
use crate::api::AppState;

// ── POST /v1/auth/publishable-keys — Create a new SDK key (authenticated via rl_live_) ──

#[utoipa::path(
    post,
    path = "/v1/auth/publishable-keys",
    tag = "Authentication",
    request_body = CreateSdkKeyRequest,
    responses(
        (status = 201, description = "SDK key created", body = CreateSdkKeyResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_sdk_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateSdkKeyRequest>,
) -> Response {
    let Some(sdk_keys_repo) = &state.sdk_keys_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Validate domain is verified and owned by tenant.
    let Some(domains_repo) = &state.domains_repo else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Domains not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let domain = match domains_repo.find_by_domain(&req.domain).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Domain not found", "code": "domain_not_found" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to look up domain: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    if domain.tenant_id != tenant.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Domain not owned by this tenant", "code": "domain_not_owned" })),
        )
            .into_response();
    }

    if !domain.verified {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Domain not verified", "code": "domain_not_verified" })),
        )
            .into_response();
    }

    let (full_key, hash, prefix) = keys::generate_sdk_key();
    let now = DateTime::now();
    let doc = SdkKeyDoc {
        id: ObjectId::new(),
        tenant_id: tenant.0,
        key_hash: hash,
        key_prefix: prefix,
        domain: req.domain.clone(),
        revoked: false,
        created_at: now,
    };

    if let Err(e) = sdk_keys_repo.create_key(&doc).await {
        tracing::error!("Failed to create SDK key: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!(CreateSdkKeyResponse {
            id: doc.id.to_hex(),
            key: full_key,
            domain: req.domain,
            created_at: now.try_to_rfc3339_string().unwrap_or_default(),
        })),
    )
        .into_response()
}

// ── GET /v1/auth/publishable-keys — List SDK keys (authenticated via rl_live_) ──

#[utoipa::path(
    get,
    path = "/v1/auth/publishable-keys",
    tag = "Authentication",
    responses(
        (status = 200, description = "List of SDK keys", body = ListSdkKeysResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_sdk_keys(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(sdk_keys_repo) = &state.sdk_keys_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match sdk_keys_repo.list_by_tenant(&tenant.0).await {
        Ok(docs) => {
            let keys: Vec<SdkKeyDetail> = docs
                .iter()
                .map(|d| SdkKeyDetail {
                    id: d.id.to_hex(),
                    key_prefix: d.key_prefix.clone(),
                    domain: d.domain.clone(),
                    created_at: d.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!(ListSdkKeysResponse { keys })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list SDK keys: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── DELETE /v1/auth/publishable-keys/{key_id} — Revoke an SDK key (authenticated via rl_live_) ──

#[utoipa::path(
    delete,
    path = "/v1/auth/publishable-keys/{key_id}",
    tag = "Authentication",
    params(("key_id" = String, Path, description = "SDK Key ID")),
    responses(
        (status = 204, description = "SDK key revoked"),
        (status = 404, description = "SDK key not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn revoke_sdk_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(key_id): Path<String>,
) -> Response {
    let Some(sdk_keys_repo) = &state.sdk_keys_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&key_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid key ID", "code": "bad_request" })),
        )
            .into_response();
    };

    match sdk_keys_repo.revoke(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "SDK key not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to revoke SDK key: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}
