use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use serde::Deserialize;

use super::models::*;
use super::repo::LinksRepository;
use crate::api::auth::middleware::{SdkDomain, TenantId};
use crate::api::domains::repo::DomainsRepository;
use crate::api::AppState;
use crate::core::validation;
use crate::core::webhook_dispatcher::{AttributionEventPayload, ClickEventPayload};

// ── Resolve Query Params ──

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    #[serde(default)]
    pub redirect: Option<String>,
}

// ── Platform Detection ──

#[derive(Debug, Clone, Copy, PartialEq)]
enum Platform {
    Ios,
    Android,
    Other,
}

impl Platform {
    fn as_str(&self) -> &'static str {
        match self {
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Other => "other",
        }
    }
}

fn detect_platform(user_agent: &str) -> Platform {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        Platform::Ios
    } else if ua.contains("android") {
        Platform::Android
    } else {
        Platform::Other
    }
}

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

            // Custom IDs require a verified custom domain.
            if !tenant_has_verified_domain(state.domains_repo.as_deref(), &tenant.0).await {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Custom IDs require a verified custom domain", "code": "no_verified_domain" })),
                )
                    .into_response();
            }

            if repo
                .find_link_by_tenant_and_id(&tenant.0, custom)
                .await
                .ok()
                .flatten()
                .is_some()
            {
                return (StatusCode::CONFLICT, Json(json!({ "error": format!("'{}' is already taken", custom), "code": "link_id_taken" }))).into_response();
            }
            custom.clone()
        }
        None => generate_link_id(),
    };

    if let Err(e) = validate_link_urls(
        req.web_url.as_deref(),
        req.ios_deep_link.as_deref(),
        req.android_deep_link.as_deref(),
        req.ios_store_url.as_deref(),
        req.android_store_url.as_deref(),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e, "code": "invalid_url" })),
        )
            .into_response();
    }

    // Check web_url against known threat feeds (only vector for phishing redirects).
    if let Some(ref web_url) = req.web_url {
        if let Some(reason) = state.threat_feed.check_url(web_url).await {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": reason, "code": "threat_detected" })),
            )
                .into_response();
        }
    }

    if let Some(ref meta) = req.metadata {
        if let Err(e) = validation::validate_metadata(meta) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e, "code": "invalid_metadata" })),
            )
                .into_response();
        }
    }

    if let Some(ref ac) = req.agent_context {
        if let Some(ref action) = ac.action {
            if let Err(e) = validation::validate_agent_action(action) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
        if let Some(ref cta) = ac.cta {
            if let Err(e) = validation::validate_cta(cta) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
        if let Some(ref desc) = ac.description {
            if let Err(e) = validation::validate_agent_description(desc) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
    }

    let metadata = req
        .metadata
        .and_then(|v| mongodb::bson::to_document(&v).ok());

    let mut input = CreateLinkInput::new(tenant.0, link_id.clone());
    if let Some(v) = req.ios_deep_link {
        input = input.ios_deep_link(v);
    }
    if let Some(v) = req.android_deep_link {
        input = input.android_deep_link(v);
    }
    if let Some(v) = req.web_url {
        input = input.web_url(v);
    }
    if let Some(v) = req.ios_store_url {
        input = input.ios_store_url(v);
    }
    if let Some(v) = req.android_store_url {
        input = input.android_store_url(v);
    }
    if let Some(v) = metadata {
        input = input.metadata(v);
    }
    if let Some(ac) = req.agent_context {
        input = input.agent_context(ac);
    }

    // Links without a verified custom domain expire after 30 days.
    let has_domain = tenant_has_verified_domain(state.domains_repo.as_deref(), &tenant.0).await;
    if !has_domain {
        let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
        input = input.expires_at(DateTime::from_millis(
            DateTime::now().timestamp_millis() + thirty_days_ms,
        ));
    }

    match repo.create_link(input).await {
        Ok(_) => {}
        Err(e) if e.contains("E11000") => {
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

// ── GET /v1/links — List links for tenant with cursor pagination (authenticated) ──

#[utoipa::path(
    get,
    path = "/v1/links",
    tag = "Links",
    params(ListLinksQuery),
    responses(
        (status = 200, description = "List of links", body = ListLinksResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_links(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Query(query): Query<ListLinksQuery>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    let cursor_id = query.cursor.and_then(|c| ObjectId::parse_str(&c).ok());

    // Fetch one extra to determine if there's a next page.
    match repo
        .list_links_by_tenant(&tenant.0, limit + 1, cursor_id)
        .await
    {
        Ok(links) => {
            let has_more = links.len() as i64 > limit;
            let page: Vec<&Link> = links.iter().take(limit as usize).collect();

            let next_cursor = if has_more {
                page.last().map(|l| l.id.to_hex())
            } else {
                None
            };

            let details: Vec<LinkDetail> = page
                .iter()
                .map(|l| LinkDetail {
                    link_id: l.link_id.clone(),
                    url: format!("{}/r/{}", state.config.public_url, l.link_id),
                    ios_deep_link: l.ios_deep_link.clone(),
                    android_deep_link: l.android_deep_link.clone(),
                    web_url: l.web_url.clone(),
                    ios_store_url: l.ios_store_url.clone(),
                    android_store_url: l.android_store_url.clone(),
                    created_at: l.created_at.try_to_rfc3339_string().unwrap_or_default(),
                    agent_context: l.agent_context.clone(),
                })
                .collect();

            Json(json!({ "links": details, "next_cursor": next_cursor })).into_response()
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

// ── PUT /v1/links/{link_id} — Update a link (authenticated) ──

#[utoipa::path(
    put,
    path = "/v1/links/{link_id}",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    request_body = UpdateLinkRequest,
    responses(
        (status = 200, description = "Link updated", body = LinkDetail),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn update_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(link_id): Path<String>,
    Json(req): Json<UpdateLinkRequest>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Flatten Option<Option<String>> to Option<&str> for validation.
    let ios_dl = req.ios_deep_link.as_ref().and_then(|v| v.as_deref());
    let android_dl = req.android_deep_link.as_ref().and_then(|v| v.as_deref());

    if let Err(e) = validate_link_urls(
        req.web_url.as_deref(),
        ios_dl,
        android_dl,
        req.ios_store_url.as_deref(),
        req.android_store_url.as_deref(),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e, "code": "invalid_url" })),
        )
            .into_response();
    }

    // Check web_url against threat feeds on update too.
    if let Some(ref web_url) = req.web_url {
        if let Some(reason) = state.threat_feed.check_url(web_url).await {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": reason, "code": "threat_detected" })),
            )
                .into_response();
        }
    }

    if let Some(ref meta) = req.metadata {
        if let Err(e) = validation::validate_metadata(meta) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e, "code": "invalid_metadata" })),
            )
                .into_response();
        }
    }

    if let Some(ref ac) = req.agent_context {
        if let Some(ref action) = ac.action {
            if let Err(e) = validation::validate_agent_action(action) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
        if let Some(ref cta) = ac.cta {
            if let Err(e) = validation::validate_cta(cta) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
        if let Some(ref desc) = ac.description {
            if let Err(e) = validation::validate_agent_description(desc) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e, "code": "invalid_agent_context" })),
                )
                    .into_response();
            }
        }
    }

    let mut update = mongodb::bson::Document::new();
    let mut unset = mongodb::bson::Document::new();

    // ios_deep_link and android_deep_link support null to clear.
    match &req.ios_deep_link {
        None => {}
        Some(None) => {
            unset.insert("ios_deep_link", "");
        }
        Some(Some(v)) => {
            update.insert("ios_deep_link", v.clone());
        }
    }
    match &req.android_deep_link {
        None => {}
        Some(None) => {
            unset.insert("android_deep_link", "");
        }
        Some(Some(v)) => {
            update.insert("android_deep_link", v.clone());
        }
    }

    if let Some(v) = &req.web_url {
        update.insert("web_url", v.clone());
    }
    if let Some(v) = &req.ios_store_url {
        update.insert("ios_store_url", v.clone());
    }
    if let Some(v) = &req.android_store_url {
        update.insert("android_store_url", v.clone());
    }
    if let Some(v) = &req.metadata {
        if let Ok(doc) = mongodb::bson::to_document(v) {
            update.insert("metadata", doc);
        }
    }
    if let Some(ref ac) = req.agent_context {
        if let Ok(doc) = mongodb::bson::to_document(ac) {
            update.insert("agent_context", doc);
        }
    }

    if update.is_empty() && unset.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No fields to update", "code": "empty_update" })),
        )
            .into_response();
    }

    match repo.update_link(&tenant.0, &link_id, update, unset).await {
        Ok(true) => {
            // Fetch updated link to return.
            let link = repo
                .find_link_by_tenant_and_id(&tenant.0, &link_id)
                .await
                .ok()
                .flatten();
            match link {
                Some(l) => Json(json!({
                    "link_id": l.link_id,
                    "url": format!("{}/r/{}", state.config.public_url, l.link_id),
                    "ios_deep_link": l.ios_deep_link,
                    "android_deep_link": l.android_deep_link,
                    "web_url": l.web_url,
                    "ios_store_url": l.ios_store_url,
                    "android_store_url": l.android_store_url,
                    "created_at": l.created_at.try_to_rfc3339_string().unwrap_or_default(),
                    "agent_context": l.agent_context,
                }))
                .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Link not found", "code": "not_found" })),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update link: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── DELETE /v1/links/{link_id} — Delete a link (authenticated) ──

#[utoipa::path(
    delete,
    path = "/v1/links/{link_id}",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 204, description = "Link deleted"),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_link(
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

    match repo.delete_link(&tenant.0, &link_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete link: {e}");
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

    let Some(_link) = repo
        .find_link_by_tenant_and_id(&tenant.0, &link_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

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
    Query(query): Query<ResolveQuery>,
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

    if let Some(resp) = check_link_resolvable(&link) {
        return resp;
    }

    let redirect = query.redirect.as_deref() == Some("1");

    do_resolve(&state, repo.as_ref(), link, &link_id, &headers, redirect).await
}

// ── GET /{link_id} — Resolve via custom domain (public) ──

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
    Query(query): Query<ResolveQuery>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_link_id(&link_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    let host = headers
        .get("x-rift-host")
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

    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo
        .find_link_by_tenant_and_id(&domain.tenant_id, &link_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if let Some(resp) = check_link_resolvable(&link) {
        return resp;
    }

    let redirect = query.redirect.as_deref() == Some("1");

    do_resolve(&state, repo.as_ref(), link, &link_id, &headers, redirect).await
}

/// Check if a link is resolvable (active, not expired, not flagged/disabled).
/// Returns None if ok, Some(response) if the link should not be resolved.
fn check_link_resolvable(link: &Link) -> Option<Response> {
    // Check expiry.
    if let Some(expires_at) = link.expires_at {
        if DateTime::now().timestamp_millis() > expires_at.timestamp_millis() {
            return Some(
                (
                    StatusCode::GONE,
                    Json(json!({ "error": "Link has expired", "code": "link_expired" })),
                )
                    .into_response(),
            );
        }
    }

    // Check status.
    match link.status {
        LinkStatus::Active => None,
        LinkStatus::Flagged => {
            let reason = link.flag_reason.as_deref().unwrap_or("potentially harmful");
            let html = format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Warning — Rift</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ background: #0a0a0a; color: #fafafa; font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; padding: 24px; }}
        .container {{ text-align: center; max-width: 480px; }}
        .icon {{ font-size: 48px; margin-bottom: 16px; }}
        h1 {{ font-size: 22px; font-weight: 600; margin-bottom: 12px; color: #f59e0b; }}
        p {{ color: #a3a3a3; font-size: 15px; line-height: 1.6; margin-bottom: 8px; }}
        .reason {{ color: #71717a; font-size: 13px; margin-top: 16px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">&#9888;</div>
        <h1>This link has been flagged</h1>
        <p>This link has been flagged as {reason_escaped}. It has been disabled for your safety.</p>
        <p class="reason">If you believe this is a mistake, contact the link owner.</p>
    </div>
</body>
</html>"#,
                reason_escaped = html_escape(reason),
            );
            Some((StatusCode::OK, axum::response::Html(html)).into_response())
        }
        LinkStatus::Disabled => Some(
            (
                StatusCode::GONE,
                Json(json!({ "error": "Link has been removed", "code": "link_disabled" })),
            )
                .into_response(),
        ),
    }
}

/// Shared resolve logic: detect platform, record click, content negotiation.
async fn do_resolve(
    state: &Arc<AppState>,
    repo: &dyn LinksRepository,
    link: Link,
    link_id: &str,
    headers: &HeaderMap,
    redirect: bool,
) -> Response {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let referer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let platform = user_agent
        .as_deref()
        .map(detect_platform)
        .unwrap_or(Platform::Other);

    if let Err(e) = repo
        .record_click(
            link.tenant_id,
            link_id,
            user_agent.clone(),
            referer.clone(),
            Some(platform.as_str().to_string()),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to record click");
    }

    if let Some(dispatcher) = &state.webhook_dispatcher {
        dispatcher.dispatch_click(ClickEventPayload {
            tenant_id: link.tenant_id.to_hex(),
            link_id: link_id.to_string(),
            user_agent,
            referer,
            platform: platform.as_str().to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    // Agents get JSON, humans get a smart landing page or redirect.
    let wants_json = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        let metadata = link
            .metadata
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        let status = compute_link_status(&link);
        let (tenant_domain, tenant_verified) =
            lookup_tenant_domain(state.domains_repo.as_deref(), &link.tenant_id).await;
        let rift_meta = build_rift_meta(status, tenant_domain, tenant_verified);

        return Json(json!({
            "link_id": link.link_id,
            "ios_deep_link": link.ios_deep_link,
            "android_deep_link": link.android_deep_link,
            "web_url": link.web_url,
            "ios_store_url": link.ios_store_url,
            "android_store_url": link.android_store_url,
            "metadata": metadata,
            "agent_context": link.agent_context,
            "_rift_meta": rift_meta,
        }))
        .into_response();
    }

    // redirect=1 mode: skip landing page, go directly to the platform destination.
    // Clipboard write happens client-side in Rift.click() (has user gesture).
    if redirect {
        match platform {
            Platform::Ios => {
                if let Some(store_url) = &link.ios_store_url {
                    return Redirect::temporary(store_url).into_response();
                }
                // No ios_store_url — fall through to landing page.
            }
            Platform::Android => {
                if let Some(store_url) = &link.android_store_url {
                    let sep = if store_url.contains('?') { "&" } else { "?" };
                    let redirect_url = format!(
                        "{}{}referrer={}",
                        store_url,
                        sep,
                        urlencoding(&format!("rift_link={}", link_id)),
                    );
                    return Redirect::temporary(&redirect_url).into_response();
                }
                // No android_store_url — fall through to landing page.
            }
            Platform::Other => {
                if let Some(web_url) = &link.web_url {
                    return Redirect::temporary(web_url).into_response();
                }
                // No web_url — fall through to landing page.
            }
        }
    }

    // If this link has per-platform destinations, serve the smart landing page.
    let has_platform_destinations = link.ios_deep_link.is_some()
        || link.android_deep_link.is_some()
        || link.ios_store_url.is_some()
        || link.android_store_url.is_some();

    // Compute link status and tenant domain for landing page and JSON-LD.
    let link_status = compute_link_status(&link);
    let (tenant_domain, tenant_verified) =
        lookup_tenant_domain(state.domains_repo.as_deref(), &link.tenant_id).await;

    if has_platform_destinations {
        // Fetch app branding from apps_repo, preferring the detected platform.
        let (app_name, icon_url, theme_color) = if let Some(apps_repo) = &state.apps_repo {
            let preferred = match platform {
                Platform::Ios => "ios",
                Platform::Android => "android",
                Platform::Other => "ios",
            };
            let fallback = match platform {
                Platform::Ios => "android",
                Platform::Android => "ios",
                Platform::Other => "android",
            };
            let app = apps_repo
                .find_by_tenant_platform(&link.tenant_id, preferred)
                .await
                .ok()
                .flatten()
                .or(apps_repo
                    .find_by_tenant_platform(&link.tenant_id, fallback)
                    .await
                    .ok()
                    .flatten());
            (
                app.as_ref().and_then(|a| a.app_name.clone()),
                app.as_ref().and_then(|a| a.icon_url.clone()),
                app.as_ref().and_then(|a| a.theme_color.clone()),
            )
        } else {
            (None, None, None)
        };

        // Extract link-level metadata for OG tags.
        let meta_title = link
            .metadata
            .as_ref()
            .and_then(|d| d.get_str("title").ok())
            .map(|s| s.to_string());
        let meta_description = link
            .metadata
            .as_ref()
            .and_then(|d| d.get_str("description").ok())
            .map(|s| s.to_string());
        let meta_image = link
            .metadata
            .as_ref()
            .and_then(|d| d.get_str("image").ok())
            .map(|s| s.to_string());

        let html = render_smart_landing_page(&LandingPageContext {
            platform,
            link: &link,
            link_id,
            app_name: app_name.as_deref(),
            icon_url: icon_url.as_deref(),
            theme_color: theme_color.as_deref(),
            meta_title: meta_title.as_deref(),
            meta_description: meta_description.as_deref(),
            meta_image: meta_image.as_deref(),
            agent_context: link.agent_context.as_ref(),
            link_status,
            tenant_domain: tenant_domain.as_deref(),
            tenant_verified,
        });
        return (StatusCode::OK, axum::response::Html(html)).into_response();
    }

    // Plain web_url redirect or minimal page.
    match &link.web_url {
        Some(url) => Redirect::temporary(url).into_response(),
        None => {
            let html = format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rift — {link_id}</title>
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
        <div class="logo">Rift</div>
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

// ── POST /v1/attribution/click — SDK-authenticated click ──

#[utoipa::path(
    post,
    path = "/v1/attribution/click",
    tag = "Attribution",
    request_body = ClickRequest,
    responses(
        (status = 200, description = "Click recorded, link data returned"),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, headers))]
pub async fn attribution_click(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(sdk_domain): axum::Extension<SdkDomain>,
    headers: HeaderMap,
    Json(req): Json<ClickRequest>,
) -> Response {
    tracing::debug!(domain = %sdk_domain.0, "SDK click via domain");

    if req.link_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "link_id is required", "code": "bad_request" })),
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

    // Tenant-scoped lookup via the SDK key's tenant.
    let link = repo
        .find_link_by_tenant_and_id(&tenant.0, &req.link_id)
        .await
        .ok()
        .flatten();

    let Some(link) = link else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    if let Some(resp) = check_link_resolvable(&link) {
        return resp;
    }

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let referer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let platform = user_agent
        .as_deref()
        .map(detect_platform)
        .unwrap_or(Platform::Other);

    if let Err(e) = repo
        .record_click(
            link.tenant_id,
            &req.link_id,
            user_agent.clone(),
            referer.clone(),
            Some(platform.as_str().to_string()),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to record click");
    }

    if let Some(dispatcher) = &state.webhook_dispatcher {
        dispatcher.dispatch_click(ClickEventPayload {
            tenant_id: link.tenant_id.to_hex(),
            link_id: req.link_id.clone(),
            user_agent,
            referer,
            platform: platform.as_str().to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    let metadata = link
        .metadata
        .as_ref()
        .and_then(|d| serde_json::to_value(d).ok());

    Json(json!({
        "link_id": req.link_id,
        "platform": platform.as_str(),
        "ios_deep_link": link.ios_deep_link,
        "android_deep_link": link.android_deep_link,
        "web_url": link.web_url,
        "ios_store_url": link.ios_store_url,
        "android_store_url": link.android_store_url,
        "metadata": metadata,
        "agent_context": link.agent_context,
    }))
    .into_response()
}

// ── POST /v1/attribution/report — SDK-authenticated attribution report ──

#[utoipa::path(
    post,
    path = "/v1/attribution/report",
    tag = "Attribution",
    request_body = AttributionReportRequest,
    responses(
        (status = 200, description = "Attribution recorded"),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn attribution_report(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<AttributionReportRequest>,
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

    let link = repo
        .find_link_by_tenant_and_id(&tenant.0, &req.link_id)
        .await
        .ok()
        .flatten();

    let Some(link) = link else {
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

    if let Some(dispatcher) = &state.webhook_dispatcher {
        dispatcher.dispatch_attribution(AttributionEventPayload {
            tenant_id: link.tenant_id.to_hex(),
            link_id: req.link_id.clone(),
            install_id: req.install_id.clone(),
            app_version: req.app_version.clone(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    Json(json!({ "success": true })).into_response()
}

// ── GET /v1/links/{link_id}/timeseries — Click timeseries (authenticated) ──

#[utoipa::path(
    get,
    path = "/v1/links/{link_id}/timeseries",
    tag = "Links",
    params(
        ("link_id" = String, Path, description = "Link ID"),
        TimeseriesQuery,
    ),
    responses(
        (status = 200, description = "Timeseries data", body = TimeseriesResponse),
        (status = 400, description = "Invalid parameters", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_link_timeseries(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(link_id): Path<String>,
    Query(query): Query<TimeseriesQuery>,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Validate granularity.
    let granularity = query.granularity.as_deref().unwrap_or("daily");
    if granularity != "daily" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Only 'daily' granularity is supported", "code": "invalid_granularity" })),
        )
            .into_response();
    }

    // Parse date range.
    let now = DateTime::now();
    let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
    let ninety_days_ms = 90 * 24 * 60 * 60 * 1000_i64;

    let from = match &query.from {
        Some(s) => DateTime::parse_rfc3339_str(s).map_err(|_| ()).or_else(|_| {
            // Try date-only format: "2026-03-01" -> "2026-03-01T00:00:00Z"
            DateTime::parse_rfc3339_str(format!("{s}T00:00:00Z")).map_err(|_| ())
        }),
        None => Ok(DateTime::from_millis(
            now.timestamp_millis() - thirty_days_ms,
        )),
    };
    let to = match &query.to {
        Some(s) => DateTime::parse_rfc3339_str(s)
            .map_err(|_| ())
            .or_else(|_| DateTime::parse_rfc3339_str(format!("{s}T23:59:59Z")).map_err(|_| ())),
        None => Ok(now),
    };

    let (from, to) = match (from, to) {
        (Ok(f), Ok(t)) => (f, t),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid date format. Use RFC 3339 (e.g. 2026-03-01T00:00:00Z) or YYYY-MM-DD", "code": "invalid_date" })),
            )
                .into_response();
        }
    };

    // Validate range.
    if to.timestamp_millis() - from.timestamp_millis() > ninety_days_ms {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Date range cannot exceed 90 days", "code": "range_too_large" })),
        )
            .into_response();
    }

    // Verify link exists for this tenant.
    if repo
        .find_link_by_tenant_and_id(&tenant.0, &link_id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    }

    match repo
        .get_click_timeseries(&tenant.0, &link_id, from, to)
        .await
    {
        Ok(data) => Json(json!({
            "link_id": link_id,
            "granularity": granularity,
            "from": from.try_to_rfc3339_string().unwrap_or_default(),
            "to": to.try_to_rfc3339_string().unwrap_or_default(),
            "data": data,
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to get timeseries: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
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

// ── Smart Landing Page ──

struct LandingPageContext<'a> {
    platform: Platform,
    link: &'a Link,
    link_id: &'a str,
    app_name: Option<&'a str>,
    icon_url: Option<&'a str>,
    theme_color: Option<&'a str>,
    meta_title: Option<&'a str>,
    meta_description: Option<&'a str>,
    meta_image: Option<&'a str>,
    agent_context: Option<&'a AgentContext>,
    link_status: &'a str,
    tenant_domain: Option<&'a str>,
    tenant_verified: bool,
}

fn render_smart_landing_page(ctx: &LandingPageContext) -> String {
    let app_name_display = ctx.app_name.unwrap_or("App");
    let theme = ctx.theme_color.unwrap_or("#0d9488");
    let link_id_js = js_escape(ctx.link_id);
    let platform = ctx.platform;
    let link = ctx.link;
    let platform_js = js_escape(platform.as_str());

    let deep_link = match platform {
        Platform::Ios => link.ios_deep_link.as_deref().unwrap_or(""),
        Platform::Android => link.android_deep_link.as_deref().unwrap_or(""),
        Platform::Other => "",
    };
    let deep_link_js = js_escape(deep_link);

    let store_url = match platform {
        Platform::Ios => link.ios_store_url.as_deref().unwrap_or(""),
        Platform::Android => link.android_store_url.as_deref().unwrap_or(""),
        Platform::Other => "",
    };

    // For Android, append referrer with link_id to store URL.
    let store_url_with_referrer = if platform == Platform::Android && !store_url.is_empty() {
        let sep = if store_url.contains('?') { "&" } else { "?" };
        format!(
            "{}{}referrer={}",
            store_url,
            sep,
            urlencoding(&format!("rift_link={}", ctx.link_id))
        )
    } else {
        store_url.to_string()
    };
    let store_url_js = js_escape(&store_url_with_referrer);

    let web_url = link.web_url.as_deref().unwrap_or("");
    let web_url_js = js_escape(web_url);

    let og_title = ctx.meta_title.unwrap_or(app_name_display);
    let og_description = ctx.meta_description.unwrap_or("Open in app");

    let json_ld = if let Some(ac) = ctx.agent_context {
        if ac.action.is_some() || ac.cta.is_some() || ac.description.is_some() {
            let action_type = ac
                .action
                .as_deref()
                .map(action_to_schema_type)
                .unwrap_or("ViewAction");

            let mut entry_points = Vec::new();
            if let Some(dl) = &ctx.link.ios_deep_link {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": dl,
                    "actionPlatform": "http://schema.org/IOSPlatform"
                }));
            }
            if let Some(dl) = &ctx.link.android_deep_link {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": dl,
                    "actionPlatform": "http://schema.org/AndroidPlatform"
                }));
            }
            if let Some(url) = &ctx.link.web_url {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": url,
                    "actionPlatform": "http://schema.org/DesktopWebPlatform"
                }));
            }

            let mut action = json!({
                "@context": "https://schema.org",
                "@type": action_type,
            });
            if let Some(cta) = &ac.cta {
                action["name"] = json!(cta);
            }
            if let Some(desc) = &ac.description {
                action["description"] = json!(desc);
            }
            if !entry_points.is_empty() {
                action["target"] = json!(entry_points);
            }

            // Add product info from metadata if available.
            if ctx.meta_title.is_some() || ctx.meta_description.is_some() {
                let mut product = json!({"@type": "Product"});
                if let Some(t) = ctx.meta_title {
                    product["name"] = json!(t);
                }
                if let Some(d) = ctx.meta_description {
                    product["description"] = json!(d);
                }
                action["object"] = product;
            }

            // Add provenance metadata.
            action["provider"] = json!({
                "@type": "Organization",
                "name": ctx.tenant_domain.unwrap_or("unknown"),
                "additionalProperty": [
                    { "@type": "PropertyValue", "name": "status", "value": ctx.link_status },
                    { "@type": "PropertyValue", "name": "verified", "value": ctx.tenant_verified },
                ]
            });

            let json_str = serde_json::to_string(&action).unwrap_or_default();
            // Escape </script> in JSON-LD to prevent XSS.
            let json_str = json_str.replace("</", "<\\/");
            format!(
                r#"    <script type="application/ld+json">{}</script>"#,
                json_str
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let og_image_tag = ctx
        .meta_image
        .map(|img| {
            format!(
                r#"    <meta property="og:image" content="{}" />"#,
                html_escape(img)
            )
        })
        .unwrap_or_default();

    let icon_html = ctx
        .icon_url
        .map(|url| {
            format!(
                r#"<img src="{}" alt="{}" style="width:64px;height:64px;border-radius:14px;margin-bottom:16px;" />"#,
                html_escape(url),
                html_escape(app_name_display),
            )
        })
        .unwrap_or_default();

    let title_html = ctx
        .meta_title
        .map(|t| {
            format!(
                r#"<h1 style="font-size:20px;font-weight:600;margin-bottom:8px;">{}</h1>"#,
                html_escape(t)
            )
        })
        .unwrap_or_default();

    let desc_html = ctx
        .meta_description
        .map(|d| {
            format!(
                r#"<p style="color:#a3a3a3;font-size:14px;margin-bottom:8px;">{}</p>"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

    let agent_description = ctx.agent_context.and_then(|ac| ac.description.as_deref());

    let meta_desc_tag = agent_description
        .or(ctx.meta_description)
        .map(|d| {
            format!(
                r#"    <meta name="description" content="{}" />"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

    let agent_panel = build_agent_panel(ctx);

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{og_title} — Rift</title>
    <meta property="og:title" content="{og_title_escaped}" />
    <meta property="og:description" content="{og_desc_escaped}" />
{meta_desc_tag}
{og_image_tag}
{json_ld}
    <style>
        *,*::before,*::after {{ box-sizing:border-box; margin:0; padding:0; }}
        body {{ font-family:system-ui,-apple-system,sans-serif; background:#0a0a0a; color:#fafafa; min-height:100vh; display:flex; flex-direction:column; }}
        .split {{ display:flex; flex:1; min-height:100vh; }}
        .side-human {{ width:60%; display:flex; align-items:center; justify-content:center; padding:48px 40px; border-right:1px solid #1e1e22; }}
        .side-agent {{ width:40%; background:#0d0d0f; padding:36px 28px; display:flex; flex-direction:column; overflow-y:auto; }}
        .human-inner {{ text-align:center; max-width:320px; }}
        .brand {{ font-size:11px; font-weight:700; letter-spacing:3px; text-transform:uppercase; color:{theme}; margin-bottom:20px; }}
        .human-inner h1 {{ font-size:22px; font-weight:700; line-height:1.3; margin-bottom:8px; }}
        .human-inner .subtitle {{ font-size:14px; color:#71717a; margin-bottom:32px; }}
        .btn {{ display:inline-block; background:{theme}; color:#fff; font-size:15px; font-weight:600; padding:14px 40px; border-radius:10px; text-decoration:none; }}
        .btn:hover {{ opacity:0.9; }}
        .sub {{ color:#737373; font-size:12px; margin-top:16px; }}
        .badge {{ display:inline-flex; align-items:center; gap:8px; background:rgba(13,148,136,0.08); border:1px solid rgba(13,148,136,0.25); border-radius:20px; padding:6px 14px; font-size:12px; font-weight:600; color:{theme}; margin-bottom:8px; width:fit-content; }}
        .badge svg {{ flex-shrink:0; }}
        .agent-tagline {{ font-size:13px; color:#52525b; margin-bottom:24px; }}
        .trust-group {{ border-radius:10px; background:#111113; border:1px solid #1e1e22; padding:16px 18px; margin-bottom:12px; }}
        .trust-group-header {{ font-size:10px; font-weight:700; letter-spacing:1.5px; text-transform:uppercase; margin-bottom:12px; display:flex; align-items:center; gap:8px; }}
        .trust-verified {{ border-left:3px solid {theme}; }}
        .trust-verified .trust-group-header {{ color:{theme}; }}
        .trust-creator {{ border-left:3px solid #3f3f46; }}
        .trust-creator .trust-group-header {{ color:#71717a; }}
        .trust-row {{ display:flex; align-items:baseline; margin-bottom:8px; font-size:13px; line-height:1.5; }}
        .trust-row:last-child {{ margin-bottom:0; }}
        .trust-label {{ color:#71717a; min-width:80px; flex-shrink:0; font-size:12px; }}
        .trust-value {{ color:#fafafa; font-size:13px; }}
        .trust-value .check {{ color:{theme}; margin-left:4px; }}
        .status-dot {{ display:inline-block; width:7px; height:7px; border-radius:50%; background:#22c55e; margin-right:6px; vertical-align:middle; position:relative; top:-1px; }}
        .status-dot.expired {{ background:#ef4444; }}
        .status-dot.flagged {{ background:#f59e0b; }}
        .desc-block {{ margin-top:10px; padding:12px 14px; background:#0d0d0f; border-radius:8px; border:1px solid #1e1e22; font-size:12px; line-height:1.65; color:#a1a1aa; }}
        .attr-note {{ font-size:11px; color:#52525b; margin-top:8px; font-style:italic; }}
        .dest-section {{ margin-top:8px; }}
        .dest-header {{ font-size:11px; font-weight:700; color:#71717a; text-transform:uppercase; letter-spacing:1px; margin-bottom:10px; }}
        .dest-item {{ display:flex; align-items:center; gap:8px; background:#111113; border:1px solid #1e1e22; border-radius:8px; padding:10px 14px; font-size:12px; margin-bottom:6px; }}
        .dest-type {{ color:#71717a; min-width:70px; flex-shrink:0; font-weight:500; }}
        .dest-arrow {{ color:#3f3f46; }}
        .dest-url {{ color:{theme}; text-decoration:none; word-break:break-all; }}
        .dest-url:hover {{ text-decoration:underline; }}
        .agent-footer {{ margin-top:auto; padding-top:24px; }}
        .agent-footer .powered {{ font-size:12px; color:#52525b; }}
        .agent-footer .powered a {{ color:#71717a; text-decoration:none; }}
        .agent-footer .powered a:hover {{ color:{theme}; }}
        .agent-footer .hint {{ font-size:10px; color:#3f3f46; margin-top:6px; }}
        @media (max-width:767px) {{
            .split {{ flex-direction:column; min-height:auto; }}
            .side-human {{ width:100%; border-right:none; border-bottom:1px solid #1e1e22; padding:56px 24px; min-height:55vh; }}
            .side-agent {{ width:100%; padding:28px 20px; }}
        }}
    </style>
</head>
<body>
<div class="split">
    <div class="side-human">
        <div class="human-inner">
            {icon_html}
            <div class="brand">{app_name_escaped}</div>
            {title_html}
            {desc_html}
            <a id="open-btn" class="btn" href="#">Open in {app_name_escaped}</a>
            <p class="sub" id="fallback-msg"></p>
        </div>
    </div>
    <div class="side-agent">
        {agent_panel}
    </div>
</div>
    <script>
    (function() {{
        var platform = "{platform_js}";
        var deepLink = "{deep_link_js}";
        var storeUrl = "{store_url_js}";
        var webUrl = "{web_url_js}";
        var linkId = "{link_id_js}";

        var btn = document.getElementById("open-btn");
        var msg = document.getElementById("fallback-msg");

        // Copy link URL to clipboard on button tap (requires user gesture).
        btn.addEventListener("click", function() {{
            if (navigator.clipboard) {{
                navigator.clipboard.writeText(window.location.href).catch(function(){{}});
            }}
        }});

        if (platform === "ios" || platform === "android") {{
            if (deepLink) {{
                btn.href = deepLink;
                btn.addEventListener("click", function() {{
                    setTimeout(function() {{
                        if (storeUrl) {{ window.location.href = storeUrl; }}
                    }}, 1500);
                }});
            }} else if (storeUrl) {{
                btn.href = storeUrl;
                btn.textContent = "Get {app_name_escaped}";
            }} else if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }}
        }} else {{
            if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }} else if (storeUrl) {{
                btn.href = storeUrl;
                btn.textContent = "Get {app_name_escaped}";
            }}
        }}
    }})();
    </script>
</body>
</html>"##,
        og_title = html_escape(og_title),
        og_title_escaped = html_escape(og_title),
        og_desc_escaped = html_escape(og_description),
        meta_desc_tag = meta_desc_tag,
        og_image_tag = og_image_tag,
        json_ld = json_ld,
        theme = html_escape(theme),
        icon_html = icon_html,
        app_name_escaped = html_escape(app_name_display),
        title_html = title_html,
        desc_html = desc_html,
        agent_panel = agent_panel,
        platform_js = platform_js,
        deep_link_js = deep_link_js,
        store_url_js = store_url_js,
        web_url_js = web_url_js,
        link_id_js = link_id_js,
    )
}

// ── GET /llms.txt — Machine-readable link context for LLMs ──

#[tracing::instrument]
pub async fn llms_txt() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("llms.txt"),
    )
}

// ── Helpers ──

async fn tenant_has_verified_domain(
    domains_repo: Option<&dyn DomainsRepository>,
    tenant_id: &ObjectId,
) -> bool {
    let Some(repo) = domains_repo else {
        return false;
    };
    repo.list_by_tenant(tenant_id)
        .await
        .ok()
        .map(|domains| domains.iter().any(|d| d.verified))
        .unwrap_or(false)
}

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

fn validate_link_urls(
    web_url: Option<&str>,
    ios_deep_link: Option<&str>,
    android_deep_link: Option<&str>,
    ios_store_url: Option<&str>,
    android_store_url: Option<&str>,
) -> Result<(), String> {
    if let Some(v) = web_url {
        validation::validate_web_url(v).map_err(|e| format!("web_url: {e}"))?;
    }
    if let Some(v) = ios_deep_link {
        validation::validate_deep_link(v).map_err(|e| format!("ios_deep_link: {e}"))?;
    }
    if let Some(v) = android_deep_link {
        validation::validate_deep_link(v).map_err(|e| format!("android_deep_link: {e}"))?;
    }
    if let Some(v) = ios_store_url {
        validation::validate_store_url(v).map_err(|e| format!("ios_store_url: {e}"))?;
    }
    if let Some(v) = android_store_url {
        validation::validate_store_url(v).map_err(|e| format!("android_store_url: {e}"))?;
    }
    Ok(())
}

fn js_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '<' => out.push_str("\\x3c"),
            '>' => out.push_str("\\x3e"),
            '&' => out.push_str("\\x26"),
            '/' => out.push_str("\\/"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn build_agent_panel(ctx: &LandingPageContext) -> String {
    let ac = ctx.agent_context;
    let link = ctx.link;
    let theme = ctx.theme_color.unwrap_or("#0d9488");

    let mut html = String::new();

    // Badge
    html.push_str(&format!(
        r#"<div class="badge"><svg width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3" y="4" width="10" height="8" rx="2" stroke="{theme}" stroke-width="1.4"/><circle cx="6.25" cy="8" r="1" fill="{theme}"/><circle cx="9.75" cy="8" r="1" fill="{theme}"/><line x1="5" y1="3" x2="5" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/><line x1="11" y1="3" x2="11" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/></svg>Machine-Readable Link</div>"#,
        theme = html_escape(theme)
    ));
    html.push_str(
        r#"<p class="agent-tagline">This link is structured for both humans and AI agents.</p>"#,
    );

    // Verified by Rift
    html.push_str(r#"<div class="trust-group trust-verified"><div class="trust-group-header">Verified by Rift</div>"#);
    if let Some(domain) = ctx.tenant_domain {
        let check = if ctx.tenant_verified {
            r#"<span class="check">&#10003;</span>"#
        } else {
            ""
        };
        html.push_str(&format!(
            r#"<div class="trust-row"><span class="trust-label">Domain</span><span class="trust-value">{}{}</span></div>"#,
            html_escape(domain), check
        ));
    }
    let status_class = match ctx.link_status {
        "expired" => " expired",
        "flagged" => " flagged",
        _ => "",
    };
    html.push_str(&format!(
        r#"<div class="trust-row"><span class="trust-label">Status</span><span class="trust-value"><span class="status-dot{}"></span>{}</span></div>"#,
        status_class,
        html_escape(&ctx.link_status[..1].to_uppercase()) + &ctx.link_status[1..]
    ));
    html.push_str("</div>");

    // Provided by creator
    if ac.is_some_and(|a| a.action.is_some() || a.cta.is_some() || a.description.is_some()) {
        let ac = ac.unwrap();
        html.push_str(r#"<div class="trust-group trust-creator"><div class="trust-group-header">Provided by link creator</div>"#);
        if let Some(action) = &ac.action {
            html.push_str(&format!(
                r#"<div class="trust-row"><span class="trust-label">Action</span><span class="trust-value">{}</span></div>"#,
                html_escape(action)
            ));
        }
        if let Some(cta) = &ac.cta {
            html.push_str(&format!(
                r#"<div class="trust-row"><span class="trust-label">CTA</span><span class="trust-value">{}</span></div>"#,
                html_escape(cta)
            ));
        }
        if let Some(desc) = &ac.description {
            html.push_str(&format!(
                r#"<div class="desc-block">{}</div>"#,
                html_escape(desc)
            ));
            if let Some(domain) = ctx.tenant_domain {
                html.push_str(&format!(
                    r#"<p class="attr-note">Provided by the owner of {}. Not independently verified.</p>"#,
                    html_escape(domain)
                ));
            }
        }
        html.push_str("</div>");
    }

    // Destinations
    let mut dests = Vec::new();
    if let Some(v) = &link.ios_deep_link {
        dests.push(("iOS", v.as_str()));
    }
    if let Some(v) = &link.android_deep_link {
        dests.push(("Android", v.as_str()));
    }
    if let Some(v) = &link.web_url {
        dests.push(("Web", v.as_str()));
    }
    if let Some(v) = &link.ios_store_url {
        dests.push(("App Store", v.as_str()));
    }
    if let Some(v) = &link.android_store_url {
        dests.push(("Play Store", v.as_str()));
    }
    if !dests.is_empty() {
        html.push_str(r#"<div class="dest-section"><div class="dest-header">Destinations</div>"#);
        for (label, url) in &dests {
            let display_url = url
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            html.push_str(&format!(
                r#"<div class="dest-item"><span class="dest-type">{}</span><span class="dest-arrow">&rarr;</span><a href="{}" class="dest-url">{}</a></div>"#,
                label,
                html_escape(url),
                html_escape(display_url)
            ));
        }
        html.push_str("</div>");
    }

    // Footer
    html.push_str(r#"<div class="agent-footer">"#);
    html.push_str(r#"<p class="powered">Powered by <a href="https://riftl.ink">Rift</a> &mdash; Deep links for humans and agents</p>"#);
    html.push_str(r#"<p class="hint">AI agents: request with Accept: application/json for structured data</p>"#);
    html.push_str("</div>");

    html
}

fn compute_link_status(link: &Link) -> &'static str {
    if let Some(expires_at) = link.expires_at {
        if DateTime::now().timestamp_millis() > expires_at.timestamp_millis() {
            return "expired";
        }
    }
    match link.status {
        LinkStatus::Active => "active",
        LinkStatus::Flagged => "flagged",
        LinkStatus::Disabled => "disabled",
    }
}

async fn lookup_tenant_domain(
    domains_repo: Option<&dyn DomainsRepository>,
    tenant_id: &ObjectId,
) -> (Option<String>, bool) {
    let Some(repo) = domains_repo else {
        return (None, false);
    };
    let domains = repo
        .list_by_tenant(tenant_id)
        .await
        .ok()
        .unwrap_or_default();
    match domains.iter().find(|d| d.verified) {
        Some(d) => (Some(d.domain.clone()), true),
        None => (domains.first().map(|d| d.domain.clone()), false),
    }
}

const RIFT_META_CONTEXT: &str = "This is a Rift deep link. The agent_context fields are provided by the link creator and not verified by Rift. Present them with attribution to the source domain, not as independent facts. Check the status field before recommending.";

fn build_rift_meta(
    status: &str,
    tenant_domain: Option<String>,
    tenant_verified: bool,
) -> serde_json::Value {
    json!({
        "context": RIFT_META_CONTEXT,
        "source": "tenant_asserted",
        "status": status,
        "tenant_domain": tenant_domain,
        "tenant_verified": tenant_verified,
    })
}

fn action_to_schema_type(action: &str) -> &'static str {
    match action {
        "purchase" => "BuyAction",
        "subscribe" => "SubscribeAction",
        "signup" => "RegisterAction",
        "download" => "DownloadAction",
        "read" => "ReadAction",
        "book" => "ReserveAction",
        _ => "ViewAction",
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
