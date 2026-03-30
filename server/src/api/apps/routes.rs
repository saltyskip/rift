use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::middleware::TenantId;
use crate::app::AppState;
use crate::services::apps::models::*;

// ── POST /v1/apps — Register an app (iOS or Android) ──

#[utoipa::path(
    post,
    path = "/v1/apps",
    tag = "Apps",
    request_body = CreateAppRequest,
    responses(
        (status = 201, description = "App registered", body = AppDetail),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_app(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateAppRequest>,
) -> Response {
    let Some(repo) = &state.apps_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let platform = req.platform.to_lowercase();
    if platform != "ios" && platform != "android" {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "platform must be 'ios' or 'android'", "code": "invalid_platform" }))).into_response();
    }

    if platform == "ios" && (req.bundle_id.is_none() || req.team_id.is_none()) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "iOS apps require bundle_id and team_id", "code": "missing_fields" }))).into_response();
    }

    if platform == "android" && req.package_name.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Android apps require package_name", "code": "missing_fields" })),
        )
            .into_response();
    }

    if let Some(ref color) = req.theme_color {
        if let Err(e) = crate::core::validation::validate_hex_color(color) {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "error": format!("theme_color: {e}"), "code": "invalid_theme_color" }),
                ),
            )
                .into_response();
        }
    }

    if let Some(ref url) = req.icon_url {
        if let Err(e) = crate::core::validation::validate_web_url(url) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("icon_url: {e}"), "code": "invalid_icon_url" })),
            )
                .into_response();
        }
    }

    let app = crate::services::apps::models::App {
        id: ObjectId::new(),
        tenant_id: tenant.0,
        platform: platform.clone(),
        bundle_id: req.bundle_id,
        team_id: req.team_id,
        package_name: req.package_name,
        sha256_fingerprints: req.sha256_fingerprints,
        app_name: req.app_name,
        icon_url: req.icon_url,
        theme_color: req.theme_color,
        created_at: mongodb::bson::DateTime::now(),
    };

    match repo.create_or_update(app.clone()).await {
        Ok(created) => {
            let detail = to_detail(&created);
            (
                StatusCode::CREATED,
                Json(serde_json::to_value(detail).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create app: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── GET /v1/apps — List tenant's apps ──

#[utoipa::path(
    get,
    path = "/v1/apps",
    tag = "Apps",
    responses(
        (status = 200, description = "List of apps", body = Vec<AppDetail>),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_apps(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(repo) = &state.apps_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match repo.list_by_tenant(&tenant.0).await {
        Ok(apps) => {
            let details: Vec<AppDetail> = apps.iter().map(to_detail).collect();
            Json(json!({ "apps": details })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list apps: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── DELETE /v1/apps/{app_id} — Remove an app ──

#[utoipa::path(
    delete,
    path = "/v1/apps/{app_id}",
    tag = "Apps",
    params(("app_id" = String, Path, description = "App ID")),
    responses(
        (status = 204, description = "App deleted"),
        (status = 404, description = "App not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(app_id): Path<String>,
) -> Response {
    let Some(repo) = &state.apps_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&app_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid app_id", "code": "invalid_id" })),
        )
            .into_response();
    };

    match repo.delete_app(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "App not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete app: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── GET /.well-known/apple-app-site-association — Serve AASA (public) ──

#[utoipa::path(
    get,
    path = "/.well-known/apple-app-site-association",
    tag = "Apps",
    responses(
        (status = 200, description = "AASA JSON"),
        (status = 404, description = "No iOS app configured"),
    ),
)]
#[tracing::instrument(skip(state, headers))]
pub async fn serve_aasa(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(tenant_id) = resolve_tenant_from_host(&state, &headers).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not found", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(repo) = &state.apps_repo else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No apps configured", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(ios_app) = repo
        .find_by_tenant_platform(&tenant_id, "ios")
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No iOS app configured", "code": "not_found" })),
        )
            .into_response();
    };

    let (Some(team_id), Some(bundle_id)) = (&ios_app.team_id, &ios_app.bundle_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "iOS app missing team_id or bundle_id", "code": "not_found" })),
        )
            .into_response();
    };

    let app_id = format!("{team_id}.{bundle_id}");
    let aasa = json!({
        "applinks": {
            "apps": [],
            "details": [{
                "appID": app_id,
                "paths": ["*"]
            }]
        }
    });

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        Json(aasa),
    )
        .into_response()
}

// ── GET /.well-known/assetlinks.json — Serve Android assetlinks (public) ──

#[utoipa::path(
    get,
    path = "/.well-known/assetlinks.json",
    tag = "Apps",
    responses(
        (status = 200, description = "Asset links JSON"),
        (status = 404, description = "No Android app configured"),
    ),
)]
#[tracing::instrument(skip(state, headers))]
pub async fn serve_assetlinks(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(tenant_id) = resolve_tenant_from_host(&state, &headers).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not found", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(repo) = &state.apps_repo else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No apps configured", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(android_app) = repo
        .find_by_tenant_platform(&tenant_id, "android")
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No Android app configured", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(package_name) = &android_app.package_name else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Android app missing package_name", "code": "not_found" })),
        )
            .into_response();
    };

    let fingerprints = android_app.sha256_fingerprints.clone().unwrap_or_default();

    let assetlinks = json!([{
        "relation": ["delegate_permission/common.handle_all_urls"],
        "target": {
            "namespace": "android_app",
            "package_name": package_name,
            "sha256_cert_fingerprints": fingerprints,
        }
    }]);

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        Json(assetlinks),
    )
        .into_response()
}

// ── Helpers ──

/// Resolve tenant from X-Rift-Host header (custom header to avoid Cloudflare overwriting X-Forwarded-Host).
async fn resolve_tenant_from_host(state: &Arc<AppState>, headers: &HeaderMap) -> Option<ObjectId> {
    let host = headers
        .get("x-rift-host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase())?;

    if host == state.config.primary_domain {
        return None;
    }

    let domains_repo = state.domains_repo.as_ref()?;
    let domain = domains_repo.find_by_domain(&host).await.ok()??;

    if !domain.verified {
        return None;
    }

    Some(domain.tenant_id)
}

fn to_detail(app: &crate::services::apps::models::App) -> AppDetail {
    AppDetail {
        id: app.id.to_hex(),
        platform: app.platform.clone(),
        bundle_id: app.bundle_id.clone(),
        team_id: app.team_id.clone(),
        package_name: app.package_name.clone(),
        sha256_fingerprints: app.sha256_fingerprints.clone(),
        app_name: app.app_name.clone(),
        icon_url: app.icon_url.clone(),
        theme_color: app.theme_color.clone(),
        created_at: app.created_at.try_to_rfc3339_string().unwrap_or_default(),
    }
}
