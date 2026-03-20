use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use super::models::*;
use super::repo::LinksRepository;
use crate::api::auth::middleware::TenantId;
use crate::api::AppState;

// ── POST /v1/links — Create a new deep link (authenticated) ──

#[utoipa::path(
    post,
    path = "/v1/links",
    tag = "Links",
    request_body = CreateLinkRequest,
    responses(
        (status = 201, description = "Link created", body = CreateLinkResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 409, description = "Link ID already taken", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateLinkRequest>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let link_id = match &req.custom_id {
        Some(custom) => {
            if let Err(e) = validate_custom_id(custom) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_custom_id" })),
                )
                    .into_response();
            }
            if repo.find_link_by_id(custom).await.ok().flatten().is_some() {
                return (StatusCode::CONFLICT, Json(json!({ "error": format!("'{}' is already taken", custom), "code": "link_id_taken" }))).into_response();
            }
            custom.clone()
        }
        None => generate_link_id(),
    };

    let metadata = req
        .metadata
        .and_then(|v| mongodb::bson::to_document(&v).ok());

    match repo
        .create_link(tenant.0, link_id.clone(), req.destination, metadata)
        .await
    {
        Ok(_) => {}
        Err(e) if e.to_string().contains("E11000") => {
            return (StatusCode::CONFLICT, Json(json!({ "error": format!("'{}' is already taken", link_id), "code": "link_id_taken" }))).into_response();
        }
        Err(e) => {
            tracing::error!("Failed to create link: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    }

    let url = format!("{}/r/{}", state.config.public_url, link_id);
    (
        StatusCode::CREATED,
        Json(json!({ "link_id": link_id, "url": url })),
    )
        .into_response()
}

// ── GET /v1/links — List links for tenant (authenticated) ──

#[utoipa::path(
    get,
    path = "/v1/links",
    tag = "Links",
    responses(
        (status = 200, description = "List of links", body = ListLinksResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_links(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match repo.list_links_by_tenant(&tenant.0).await {
        Ok(links) => {
            let details: Vec<LinkDetail> = links
                .iter()
                .map(|l| LinkDetail {
                    link_id: l.link_id.clone(),
                    url: format!("{}/r/{}", state.config.public_url, l.link_id),
                    destination: l.destination.clone(),
                    created_at: l.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!({ "links": details })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list links: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── GET /v1/links/{link_id}/stats — Conversion stats (authenticated) ──

#[utoipa::path(
    get,
    path = "/v1/links/{link_id}/stats",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 200, description = "Link stats", body = LinkStatsResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_link_stats(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(link_id): Path<String>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo.find_link_by_id(&link_id).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if link.tenant_id != tenant.0 {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    let click_count = repo.count_clicks(&tenant.0, &link_id).await.unwrap_or(0);
    let install_count = repo
        .count_attributions(&tenant.0, &link_id)
        .await
        .unwrap_or(0);
    let conversion_rate = if click_count > 0 {
        install_count as f64 / click_count as f64
    } else {
        0.0
    };

    Json(json!({
        "link_id": link_id,
        "click_count": click_count,
        "install_count": install_count,
        "conversion_rate": conversion_rate,
    }))
    .into_response()
}

// ── GET /r/{link_id} — Resolve/redirect (public) ──
// Returns JSON for agents (Accept: application/json), 302 redirect for humans.

#[utoipa::path(
    get,
    path = "/r/{link_id}",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 302, description = "Redirect to destination"),
        (status = 200, description = "Link metadata (JSON)", body = ResolvedLink),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
)]
#[tracing::instrument(skip(state, headers))]
pub async fn resolve_link(
    State(state): State<Arc<AppState>>,
    Path(link_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_link_id(&link_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo.find_link_by_id(&link_id).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    do_resolve(repo.as_ref(), link, &link_id, &headers).await
}

// ── GET /{link_id} — Resolve via custom domain (public) ──
// Only responds when X-Forwarded-Host is present and is a verified custom domain.

#[utoipa::path(
    get,
    path = "/{link_id}",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 302, description = "Redirect to destination"),
        (status = 200, description = "Link metadata (JSON)", body = ResolvedLink),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
)]
#[tracing::instrument(skip(state, headers))]
pub async fn resolve_link_custom(
    State(state): State<Arc<AppState>>,
    Path(link_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_link_id(&link_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    // Only handle custom domain requests (via X-Forwarded-Host from the edge worker).
    let host = headers
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase());

    let Some(host) = host else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if host == state.config.primary_domain {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    // Look up the custom domain.
    let Some(domains_repo) = &state.domains_repo else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    let Some(domain) = domains_repo.find_by_domain(&host).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if !domain.verified {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not verified", "code": "not_found" })),
        )
            .into_response();
    }

    // Look up the link.
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo.find_link_by_id(&link_id).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    // Ensure the link belongs to the same tenant as the domain.
    if link.tenant_id != domain.tenant_id {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    do_resolve(repo.as_ref(), link, &link_id, &headers).await
}

/// Shared resolve logic: record click + content negotiation.
async fn do_resolve(
    repo: &dyn LinksRepository,
    link: Link,
    link_id: &str,
    headers: &HeaderMap,
) -> Response {
    // Record click (fire-and-forget).
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let referer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if let Err(e) = repo
        .record_click(link.tenant_id, link_id, user_agent, referer)
        .await
    {
        tracing::warn!(error = %e, "Failed to record click");
    }

    // Agents get JSON, humans get a redirect.
    let wants_json = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        let metadata = link.metadata.and_then(|d| serde_json::to_value(&d).ok());
        return Json(json!({
            "link_id": link.link_id,
            "destination": link.destination,
            "metadata": metadata,
        }))
        .into_response();
    }

    match &link.destination {
        Some(dest) => Redirect::temporary(dest).into_response(),
        None => {
            // No destination — show a minimal page.
            let html = format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Relay — {link_id}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            background: #0a0a0a;
            color: #fafafa;
            font-family: system-ui, -apple-system, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 24px;
        }}
        .container {{ text-align: center; max-width: 400px; }}
        .logo {{ color: #0d9488; font-size: 14px; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; margin-bottom: 24px; }}
        h1 {{ font-size: 20px; font-weight: 500; margin-bottom: 8px; }}
        p {{ color: #737373; font-size: 14px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">Relay</div>
        <h1>Link: {link_id}</h1>
        <p>No destination configured for this link.</p>
    </div>
</body>
</html>"#,
                link_id = link_id,
            );
            (StatusCode::OK, axum::response::Html(html)).into_response()
        }
    }
}

// ── POST /v1/attribution — Report an install attribution (public) ──

#[utoipa::path(
    post,
    path = "/v1/attribution",
    tag = "Attribution",
    request_body = ReportAttributionRequest,
    responses(
        (status = 200, description = "Attribution recorded", body = AttributionResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
)]
#[tracing::instrument(skip(state))]
pub async fn report_attribution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReportAttributionRequest>,
) -> Response {
    if req.link_id.is_empty() || req.install_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "link_id and install_id are required", "code": "bad_request" })),
        )
            .into_response();
    }

    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Resolve tenant from the link.
    let Some(link) = repo.find_link_by_id(&req.link_id).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if let Err(e) = repo
        .upsert_attribution(
            link.tenant_id,
            &req.link_id,
            &req.install_id,
            &req.app_version,
        )
        .await
    {
        tracing::error!("Failed to upsert attribution: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    Json(json!({ "success": true })).into_response()
}

// ── PUT /v1/attribution/link — Link attribution to user (authenticated) ──

#[utoipa::path(
    put,
    path = "/v1/attribution/link",
    tag = "Attribution",
    request_body = LinkAttributionRequest,
    responses(
        (status = 200, description = "Attribution linked", body = AttributionResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn link_attribution(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<LinkAttributionRequest>,
) -> Response {
    if req.install_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "install_id is required", "code": "bad_request" })),
        )
            .into_response();
    }

    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // For now, use the install_id as user_id (caller can pass a real user_id in the future).
    let linked = repo
        .link_attribution_to_user(&tenant.0, &req.install_id, &req.install_id)
        .await;

    match linked {
        Ok(true) => Json(json!({ "success": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(
                json!({ "error": "No attribution found for this install_id", "code": "not_found" }),
            ),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to link attribution: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── Helpers ──

fn generate_link_id() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase()
}

fn is_valid_link_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn validate_custom_id(id: &str) -> Result<(), String> {
    if id.len() < 3 || id.len() > 64 {
        return Err("custom_id must be 3-64 characters".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("custom_id must be alphanumeric with hyphens only".to_string());
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err("custom_id must not start or end with a hyphen".to_string());
    }
    Ok(())
}
