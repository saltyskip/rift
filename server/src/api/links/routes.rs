use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde_json::json;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use serde::Deserialize;

use super::landing_page::{
    content_type_for_asset, demo_preview_theme, html_escape, preview_asset_path,
    render_smart_landing_page, resolve_effective_theme, LandingPageContext, ThemeResolutionInput,
};
use super::models::*;
use super::repo::LinksRepository;
use crate::api::auth::middleware::{SdkDomain, TenantId};
use crate::api::domains::models::Domain;
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

    if let Some(ref theme_override) = req.theme_override {
        if let Err(e) = validation::validate_link_theme_override(theme_override) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e, "code": "invalid_theme_override" })),
            )
                .into_response();
        }

        if let Some(theme_id) = theme_override.theme_id.as_deref() {
            let Ok(theme_id) = ObjectId::parse_str(theme_id) else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Invalid theme_id", "code": "invalid_theme_override" })),
                )
                    .into_response();
            };

            if !theme_belongs_to_tenant(&state, &tenant.0, &theme_id).await {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "theme_override.theme_id must belong to your tenant", "code": "invalid_theme_override" })),
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
    if let Some(theme_override) = req.theme_override {
        input = input.theme_override(theme_override);
    }

    // Links without a verified custom domain expire after 30 days.
    let has_domain = tenant_has_verified_domain(state.domains_repo.as_deref(), &tenant.0).await;
    let expires_at = if !has_domain {
        let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
        let expiry = DateTime::from_millis(DateTime::now().timestamp_millis() + thirty_days_ms);
        input = input.expires_at(expiry);
        Some(expiry)
    } else {
        None
    };

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
    let mut resp = json!({ "link_id": link_id, "url": url });
    if let Some(exp) = expires_at {
        resp["expires_at"] = json!(exp.try_to_rfc3339_string().unwrap_or_default());
    }
    (StatusCode::CREATED, Json(resp)).into_response()
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
                    theme_override: l.theme_override.clone(),
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

    if let Some(ref theme_override) = req.theme_override {
        if let Err(e) = validation::validate_link_theme_override(theme_override) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e, "code": "invalid_theme_override" })),
            )
                .into_response();
        }

        if let Some(theme_id) = theme_override.theme_id.as_deref() {
            let Ok(theme_id) = ObjectId::parse_str(theme_id) else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Invalid theme_id", "code": "invalid_theme_override" })),
                )
                    .into_response();
            };

            if !theme_belongs_to_tenant(&state, &tenant.0, &theme_id).await {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "theme_override.theme_id must belong to your tenant", "code": "invalid_theme_override" })),
                )
                    .into_response();
            }
        }
    }

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
    if let Some(ref theme_override) = req.theme_override {
        if let Ok(doc) = mongodb::bson::to_document(theme_override) {
            update.insert("theme_override", doc);
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
                    "theme_override": l.theme_override,
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
    description = "Content-negotiated link resolution. Browsers receive a redirect or smart landing page. Requests with `Accept: application/json` receive structured link data including `agent_context` and a `_rift_meta` trust envelope.",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 302, description = "Redirect to destination"),
        (status = 200, description = "Structured link data with agent context and trust metadata", body = ResolvedLink),
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

    do_resolve(
        &state,
        repo.as_ref(),
        link,
        &link_id,
        &headers,
        redirect,
        None,
    )
    .await
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

    do_resolve(
        &state,
        repo.as_ref(),
        link,
        &link_id,
        &headers,
        redirect,
        Some(&domain),
    )
    .await
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
    resolved_domain: Option<&Domain>,
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
            "theme_override": link.theme_override,
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
        // Fetch app identity from apps_repo, preferring the detected platform.
        let (app_name, app_icon_url) = if let Some(apps_repo) = &state.apps_repo {
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
            )
        } else {
            (None, None)
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

        let theme = resolve_effective_theme(
            state,
            &ThemeResolutionInput {
                tenant_id: &link.tenant_id,
                resolved_domain,
                link_override: link.theme_override.as_ref(),
                app_name: app_name.as_deref(),
                app_icon_url: app_icon_url.as_deref(),
                meta_title: meta_title.as_deref(),
                meta_description: meta_description.as_deref(),
                meta_image: meta_image.as_deref(),
            },
        )
        .await;

        let html = render_smart_landing_page(&LandingPageContext {
            platform_name: platform.as_str(),
            is_android: platform == Platform::Android,
            link: &link,
            link_id,
            theme: &theme,
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

#[tracing::instrument(skip(state))]
pub async fn preview_theme(
    State(state): State<Arc<AppState>>,
    Path(theme_slug): Path<String>,
) -> Response {
    let Some(theme) = demo_preview_theme(&state, &theme_slug) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Preview theme not found", "code": "not_found" })),
        )
            .into_response();
    };

    let link = Link {
        id: ObjectId::new(),
        tenant_id: ObjectId::new(),
        link_id: format!("{theme_slug}-preview"),
        ios_deep_link: Some(format!("{}://launch/featured", theme_slug.replace('-', ""))),
        android_deep_link: Some(format!("{}://launch/featured", theme_slug.replace('-', ""))),
        web_url: Some(format!("https://example.com/{theme_slug}/featured")),
        ios_store_url: Some("https://apps.apple.com/app/id123456789".to_string()),
        android_store_url: Some(
            "https://play.google.com/store/apps/details?id=com.example.preview".to_string(),
        ),
        metadata: None,
        created_at: DateTime::now(),
        status: LinkStatus::Active,
        flag_reason: None,
        expires_at: None,
        agent_context: Some(AgentContext {
            action: Some("open".to_string()),
            cta: Some("Open the featured experience".to_string()),
            description: Some(
                "Structured preview data for the themed landing page example.".to_string(),
            ),
        }),
        theme_override: None,
    };

    let meta_title = Some(theme.headline.as_str());
    let meta_description = theme.subheadline.as_deref();
    let meta_image = theme.og_image_url.as_deref();

    let html = render_smart_landing_page(&LandingPageContext {
        platform_name: Platform::Ios.as_str(),
        is_android: false,
        link: &link,
        link_id: &link.link_id,
        theme: &theme,
        meta_title,
        meta_description,
        meta_image,
        agent_context: link.agent_context.as_ref(),
        link_status: "active",
        tenant_domain: Some("preview.rift.local"),
        tenant_verified: true,
    });

    (StatusCode::OK, axum::response::Html(html)).into_response()
}

#[tracing::instrument]
pub async fn preview_asset(Path((theme_slug, asset_name)): Path<(String, String)>) -> Response {
    let Some(path) = preview_asset_path(&theme_slug, &asset_name) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Asset not found", "code": "not_found" })),
        )
            .into_response();
    };

    match fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                content_type_for_asset(&asset_name),
            )],
            bytes,
        )
            .into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Asset not found", "code": "not_found" })),
        )
            .into_response(),
    }
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

async fn theme_belongs_to_tenant(
    state: &Arc<AppState>,
    tenant_id: &ObjectId,
    theme_id: &ObjectId,
) -> bool {
    match &state.themes_repo {
        Some(repo) => repo
            .find_by_tenant_and_id(tenant_id, theme_id)
            .await
            .ok()
            .flatten()
            .is_some(),
        None => false,
    }
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
