use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use image::ImageFormat;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use qr_code_styling::config::{
    BackgroundOptions, Color, CornersDotOptions, CornersSquareOptions, DotsOptions, ImageOptions,
    QROptions,
};
use qr_code_styling::types::{
    CornerDotType, CornerSquareType, DotType, ErrorCorrectionLevel, OutputFormat, ShapeType,
};
use qr_code_styling::QRCodeStyling;
use serde_json::json;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::api::auth::middleware::{CallerScope, SdkDomain, TenantId};
use crate::app::AppState;
use crate::core::webhook_dispatcher::{AttributionEventPayload, ClickEventPayload};
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;
use crate::services::links::models::*;
use crate::services::links::service::LinkError;

// ── Resolve Query Params ──

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    #[serde(default)]
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeQuery {
    /// Logo image URL to center in the QR code.
    pub logo: Option<String>,
    /// Output size in pixels. Defaults to 600.
    #[param(example = 600)]
    pub size: Option<u32>,
    /// QR error correction level. One of L, M, Q, H. Defaults to L (or H when a logo is set).
    #[param(example = "H")]
    pub level: Option<String>,
    /// Foreground color hex value applied to dots, eye frames, and eye pupils when their
    /// per-component color is not set. Defaults to #000000.
    #[serde(rename = "fgColor")]
    #[param(example = "#111827")]
    pub fg_color: Option<String>,
    /// Background color hex value. Defaults to #FFFFFF.
    #[serde(rename = "bgColor")]
    #[param(example = "#FFFFFF")]
    pub bg_color: Option<String>,
    /// Ignore the logo URL when true.
    #[serde(default, rename = "hideLogo")]
    pub hide_logo: bool,
    /// Margin around the QR code in modules. Defaults to 2.
    #[param(example = 2)]
    pub margin: Option<u32>,
    /// Deprecated compatibility flag. If false and margin is absent, margin becomes 0.
    #[serde(rename = "includeMargin")]
    pub include_margin: Option<bool>,
    /// Shape of the inner dots. One of `square`, `dots`, `rounded`, `classy`,
    /// `classy-rounded`, `extra-rounded`. Defaults to `rounded`.
    #[serde(rename = "dotType")]
    #[param(example = "rounded")]
    pub dot_type: Option<String>,
    /// Shape of the three large positioning "eye" frames. One of `square`, `dot`,
    /// `extra-rounded`. Defaults to `extra-rounded`.
    #[serde(rename = "cornerSquareType")]
    #[param(example = "extra-rounded")]
    pub corner_square_type: Option<String>,
    /// Shape of the pupil inside each eye frame. One of `dot`, `square`. Defaults to `dot`.
    #[serde(rename = "cornerDotType")]
    #[param(example = "dot")]
    pub corner_dot_type: Option<String>,
    /// Overall canvas shape. One of `square`, `circle`. Defaults to `square`.
    #[param(example = "square")]
    pub shape: Option<String>,
    /// Override color for the inner dots only. Defaults to `fgColor`.
    #[serde(rename = "dotColor")]
    #[param(example = "#0d9488")]
    pub dot_color: Option<String>,
    /// Override color for the eye frames only. Defaults to `fgColor`.
    #[serde(rename = "cornerSquareColor")]
    #[param(example = "#111827")]
    pub corner_square_color: Option<String>,
    /// Override color for the eye pupils only. Defaults to `fgColor`.
    #[serde(rename = "cornerDotColor")]
    #[param(example = "#111827")]
    pub corner_dot_color: Option<String>,
}

fn link_error_to_response(err: LinkError) -> Response {
    // QuotaExceeded is a structured 402 — delegate to the shared helper so
    // the body shape stays consistent across every enforcement path.
    if let LinkError::QuotaExceeded(q) = err {
        return crate::api::billing::quota_response::to_response(q);
    }
    // BatchValidationFailed carries a per-row error list — return the full
    // array so the caller can fix every issue in one pass.
    if let LinkError::BatchValidationFailed(errors) = err {
        let count = errors.len();
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("{count} item(s) failed validation"),
                "code": "invalid_batch",
                "errors": errors,
            })),
        )
            .into_response();
    }
    let status = match &err {
        LinkError::InvalidCustomId(_)
        | LinkError::InvalidUrl(_)
        | LinkError::InvalidMetadata(_)
        | LinkError::InvalidAgentContext(_)
        | LinkError::InvalidSocialPreview(_)
        | LinkError::ThreatDetected(_)
        | LinkError::NoVerifiedDomain
        | LinkError::EmptyUpdate
        | LinkError::AffiliateScopeMismatch
        | LinkError::BatchTooLarge { .. }
        | LinkError::BatchEmpty
        | LinkError::BatchModeAmbiguous
        | LinkError::BatchModeMissing => StatusCode::BAD_REQUEST,
        LinkError::LinkIdTaken(_) => StatusCode::CONFLICT,
        LinkError::NotFound | LinkError::AffiliateNotFound => StatusCode::NOT_FOUND,
        LinkError::QuotaExceeded(_) | LinkError::BatchValidationFailed(_) => {
            unreachable!("handled above")
        }
        LinkError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = err.code();
    let message = err.to_string();
    (status, Json(json!({ "error": message, "code": code }))).into_response()
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
    axum::Extension(scope): axum::Extension<CallerScope>,
    Json(req): Json<CreateLinkRequest>,
) -> Response {
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Quota check + scope/affiliate enforcement live in
    // `LinksService::create_link` so MCP tool calls hit the same path.
    match svc.create_link(tenant.0, scope.0.as_ref(), req).await {
        Ok(resp) => (StatusCode::CREATED, Json(json!(resp))).into_response(),
        Err(e) => link_error_to_response(e),
    }
}

// ── POST /v1/links/bulk — Atomically create up to 100 links sharing one template ──

#[utoipa::path(
    post,
    path = "/v1/links/bulk",
    tag = "Links",
    request_body = BulkCreateLinksRequest,
    responses(
        (status = 201, description = "All links created", body = BulkCreateLinksResponse),
        (status = 400, description = "Invalid batch", body = crate::error::ErrorResponse),
        (status = 402, description = "Quota exceeded"),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_links_bulk(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(scope): axum::Extension<CallerScope>,
    Json(req): Json<BulkCreateLinksRequest>,
) -> Response {
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.create_links_bulk(tenant.0, scope.0.as_ref(), req).await {
        Ok(resp) => (StatusCode::CREATED, Json(json!(resp))).into_response(),
        Err(e) => link_error_to_response(e),
    }
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
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.list_links(&tenant.0, query.limit, query.cursor).await {
        Ok(resp) => Json(json!(resp)).into_response(),
        Err(e) => link_error_to_response(e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/links/{link_id}",
    tag = "Links",
    params(("link_id" = String, Path, description = "Link ID")),
    responses(
        (status = 200, description = "Link details", body = LinkDetail),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(scope): axum::Extension<CallerScope>,
    Path(link_id): Path<String>,
) -> Response {
    let Some(svc) = &state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.get_link(&tenant.0, scope.0.as_ref(), &link_id).await {
        Ok(detail) => Json(json!(detail)).into_response(),
        Err(crate::services::links::service::LinkError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string(), "code": e.code() })),
        )
            .into_response(),
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
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.update_link(&tenant.0, &link_id, req).await {
        Ok(detail) => Json(json!(detail)).into_response(),
        Err(e) => link_error_to_response(e),
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
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.delete_link(&tenant.0, &link_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => link_error_to_response(e),
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

    // Conversions are optional — if the repo isn't configured (no DB) or the
    // aggregation fails, return an empty list rather than failing the whole
    // stats response. The click/install counters above are the load-bearing
    // fields; conversions are additive.
    let conversions = if let Some(conv_repo) = &state.conversions_repo {
        conv_repo
            .get_conversion_counts_for_link(&tenant.0, &link_id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Json(json!({
        "link_id": link_id,
        "click_count": click_count,
        "install_count": install_count,
        "conversion_rate": conversion_rate,
        "conversions": conversions,
    }))
    .into_response()
}

// ── GET /v1/links/{link_id}/qr.{format} — Styled QR export ──
//
// Positioned as a generate-and-save endpoint: callers fetch once and embed
// the returned bytes statically. `Cache-Control: no-store` is set on success
// to discourage hotlinking, since every render re-fetches the logo and
// re-rasterizes from scratch. If hotlinking becomes a real pattern in
// telemetry, revisit with ETag/304 or a server-side cache.

#[utoipa::path(
    get,
    path = "/v1/links/{link_id}/qr.{format}",
    tag = "Links",
    params(
        ("link_id" = String, Path, description = "Link ID"),
        ("format" = String, Path, description = "Output format: `png` or `svg`", example = "png"),
        QrCodeQuery,
    ),
    responses(
        (status = 200, description = "PNG QR code", content_type = "image/png"),
        (status = 200, description = "SVG QR code", content_type = "image/svg+xml"),
        (status = 400, description = "Invalid QR format or options", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_link_qr(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path((link_id, format)): Path<(String, String)>,
    Query(query): Query<QrCodeQuery>,
) -> Response {
    let format = match format.as_str() {
        "png" => QrOutputFormat::Png,
        "svg" => QrOutputFormat::Svg,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "format must be png or svg",
                    "code": "invalid_qr_format",
                })),
            )
                .into_response();
        }
    };
    render_link_qr(state, tenant, link_id, query, format).await
}

#[derive(Debug, Clone, Copy)]
enum QrOutputFormat {
    Png,
    Svg,
}

async fn render_link_qr(
    state: Arc<AppState>,
    tenant: TenantId,
    link_id: String,
    query: QrCodeQuery,
    format: QrOutputFormat,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo
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

    let options = match QrRenderOptions::try_from_query(&query).await {
        Ok(options) => options,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": message, "code": "invalid_qr_options" })),
            )
                .into_response();
        }
    };

    let url = canonical_link_url(&state, &link).await;
    match render_qr(&url, &options, format) {
        Ok(bytes) => {
            let content_type = match format {
                QrOutputFormat::Png => "image/png",
                QrOutputFormat::Svg => "image/svg+xml; charset=utf-8",
            };
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                bytes,
            )
                .into_response()
        }
        Err(message) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": message, "code": "qr_render_error" })),
        )
            .into_response(),
    }
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

    do_resolve(&state, link, &link_id, &headers, redirect).await
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
        .or_else(|| headers.get("host"))
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

    // Alternate domain: redirect to store (no landing page, no click recording).
    // This only runs when the app is NOT installed — if it IS installed, Universal
    // Links intercept the tap and this endpoint is never reached.
    if domain.role == DomainRole::Alternate {
        let Some(links_svc) = &state.links_service else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Database not configured", "code": "no_database" })),
            )
                .into_response();
        };

        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        return match links_svc
            .resolve_alternate(&domain.tenant_id, &link_id, user_agent)
            .await
        {
            Ok(url) => Redirect::temporary(&url).into_response(),
            Err(_) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Link not found", "code": "not_found" })),
            )
                .into_response(),
        };
    }

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

    do_resolve(&state, link, &link_id, &headers, redirect).await
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

    // Quota check + retention bucket + DB write all happen inside the
    // service method so MCP / CLI / any future transport that records a
    // click can't bypass enforcement.
    if let Some(ref svc) = state.links_service {
        svc.record_click(
            link.tenant_id,
            link_id,
            user_agent.clone(),
            referer.clone(),
            Some(platform.as_str().to_string()),
        )
        .await;
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
            "social_preview": link.social_preview,
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

        // Look up alternate domain for the "Open in App" button.
        let alternate_domain = if let Some(domains_repo) = &state.domains_repo {
            domains_repo
                .find_alternate_by_tenant(&link.tenant_id)
                .await
                .ok()
                .flatten()
                .map(|d| d.domain)
        } else {
            None
        };

        let html = render_smart_landing_page(&LandingPageContext {
            platform,
            link: &link,
            link_id,
            app_name: app_name.as_deref(),
            icon_url: icon_url.as_deref(),
            theme_color: theme_color.as_deref(),
            social_preview: link.social_preview.as_ref(),
            agent_context: link.agent_context.as_ref(),
            link_status,
            tenant_domain: tenant_domain.as_deref(),
            tenant_verified,
            alternate_domain: alternate_domain.as_deref(),
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

    if let Some(ref svc) = state.links_service {
        svc.record_click(
            link.tenant_id,
            &req.link_id,
            user_agent.clone(),
            referer.clone(),
            Some(platform.as_str().to_string()),
        )
        .await;
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
        "social_preview": link.social_preview,
    }))
    .into_response()
}

// ── POST /v1/attribution/install — SDK-authenticated attribution report ──

#[utoipa::path(
    post,
    path = "/v1/attribution/install",
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

// ── PUT /v1/attribution/identify — Link attribution to user (SDK-authenticated) ──
//
// SDK-authenticated (pk_live_) because the install_id is opaque and only
// lives in the mobile SDK; no flow produces the inputs a backend would need
// to call this endpoint with a secret key.

#[utoipa::path(
    put,
    path = "/v1/attribution/identify",
    tag = "Attribution",
    request_body = LinkAttributionRequest,
    responses(
        (status = 200, description = "Attribution linked", body = AttributionResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "No attribution found for this install_id", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn link_attribution(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<LinkAttributionRequest>,
) -> Response {
    if req.install_id.is_empty() || req.user_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "install_id and user_id are required", "code": "bad_request" })),
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
        .link_attribution_to_user(&tenant.0, &req.install_id, &req.user_id)
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
    social_preview: Option<&'a SocialPreview>,
    agent_context: Option<&'a AgentContext>,
    link_status: &'a str,
    tenant_domain: Option<&'a str>,
    tenant_verified: bool,
    alternate_domain: Option<&'a str>,
}

/// Legacy fallback: before `social_preview` existed, customers used `metadata.{title,description,image}`
/// for OG tags. Read those keys when the link has no `social_preview` so existing links don't silently
/// lose their previews on deploy.
fn social_preview_from_metadata(
    metadata: Option<&mongodb::bson::Document>,
) -> Option<SocialPreview> {
    let meta = metadata?;
    let title = meta.get_str("title").ok().map(str::to_string);
    let description = meta.get_str("description").ok().map(str::to_string);
    let image_url = meta.get_str("image").ok().map(str::to_string);
    if title.is_none() && description.is_none() && image_url.is_none() {
        return None;
    }
    Some(SocialPreview {
        title,
        description,
        image_url,
    })
}

fn render_smart_landing_page(ctx: &LandingPageContext) -> String {
    let app_name_display = ctx.app_name.unwrap_or("App");
    let theme = ctx.theme_color.unwrap_or("#0d9488");
    let platform = ctx.platform;
    let link = ctx.link;
    let platform_js = js_escape(platform.as_str());

    let metadata_fallback = if ctx.social_preview.is_none() {
        social_preview_from_metadata(link.metadata.as_ref())
    } else {
        None
    };
    let effective_preview = ctx.social_preview.or(metadata_fallback.as_ref());

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

    // Alternate domain URL for the "Open in App" button (cross-domain Universal Link trigger).
    let alternate_url = ctx
        .alternate_domain
        .map(|d| format!("https://{}/{}", d, ctx.link_id))
        .unwrap_or_default();
    let alternate_url_js = js_escape(&alternate_url);

    let preview_title = effective_preview.and_then(|p| p.title.as_deref());
    let preview_description = effective_preview.and_then(|p| p.description.as_deref());
    let preview_image = effective_preview.and_then(|p| p.image_url.as_deref());
    let og_title = preview_title.unwrap_or(app_name_display);
    let og_description = preview_description.unwrap_or("Open in app");

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

            // Add public preview info if available.
            if preview_title.is_some() || preview_description.is_some() {
                let mut product = json!({"@type": "Product"});
                if let Some(t) = preview_title {
                    product["name"] = json!(t);
                }
                if let Some(d) = preview_description {
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

    let og_image_tag = preview_image
        .map(|img| {
            format!(
                r#"    <meta property="og:image" content="{img}" />
    <meta name="twitter:image" content="{img}" />"#,
                img = html_escape(img)
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

    let title_html = preview_title
        .map(|t| {
            format!(
                r#"<h1 style="font-size:20px;font-weight:600;margin-bottom:8px;">{}</h1>"#,
                html_escape(t)
            )
        })
        .unwrap_or_default();

    let desc_html = preview_description
        .map(|d| {
            format!(
                r#"<p style="color:#a3a3a3;font-size:14px;margin-bottom:8px;">{}</p>"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

    let agent_description = ctx.agent_context.and_then(|ac| ac.description.as_deref());

    let meta_desc_tag = agent_description
        .or(preview_description)
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
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content="{og_title_escaped}" />
    <meta name="twitter:description" content="{og_desc_escaped}" />
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
        var storeUrl = "{store_url_js}";
        var webUrl = "{web_url_js}";
        var alternateUrl = "{alternate_url_js}";

        var btn = document.getElementById("open-btn");
        var msg = document.getElementById("fallback-msg");

        // Copy link URL to clipboard on button tap (requires user gesture).
        btn.addEventListener("click", function() {{
            if (navigator.clipboard) {{
                navigator.clipboard.writeText(window.location.href).catch(function(){{}});
            }}
        }});

        if (platform === "ios" || platform === "android") {{
            if (alternateUrl) {{
                // Cross-domain hop triggers Universal Links / App Links.
                // If app installed → opens. If not → alternate domain redirects to store.
                btn.href = alternateUrl;
                btn.textContent = "Open in {app_name_escaped}";
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
        store_url_js = store_url_js,
        web_url_js = web_url_js,
        alternate_url_js = alternate_url_js,
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

// ── QR Rendering Helpers ──

const QR_DEFAULT_SIZE: u32 = 600;
const QR_MIN_SIZE: u32 = 128;
const QR_MAX_SIZE: u32 = 2048;
const QR_DEFAULT_MARGIN: u32 = 2;
const QR_MAX_MARGIN: u32 = 16;
const QR_MAX_LOGO_BYTES: usize = 512 * 1024;
const QR_LOGO_TIMEOUT_SECS: u64 = 5;

struct QrRenderOptions {
    size: u32,
    margin: u32,
    level: ErrorCorrectionLevel,
    fg: Color,
    bg: Color,
    logo: Option<LogoImage>,
    dot_type: DotType,
    corner_square_type: CornerSquareType,
    corner_dot_type: CornerDotType,
    shape: ShapeType,
    dot_color: Option<Color>,
    corner_square_color: Option<Color>,
    corner_dot_color: Option<Color>,
}

struct LogoImage {
    png_bytes: Vec<u8>,
}

impl QrRenderOptions {
    async fn try_from_query(query: &QrCodeQuery) -> Result<Self, String> {
        let size = query.size.unwrap_or(QR_DEFAULT_SIZE);
        if !(QR_MIN_SIZE..=QR_MAX_SIZE).contains(&size) {
            return Err(format!(
                "size must be between {QR_MIN_SIZE} and {QR_MAX_SIZE}"
            ));
        }

        let margin = query.margin.unwrap_or_else(|| {
            if query.include_margin == Some(false) {
                0
            } else {
                QR_DEFAULT_MARGIN
            }
        });
        if margin > QR_MAX_MARGIN {
            return Err(format!("margin must be between 0 and {QR_MAX_MARGIN}"));
        }

        let will_render_logo = !query.hide_logo && query.logo.is_some();
        // A centered logo covers the middle of the QR, so force max error correction when the
        // caller didn't pick one — otherwise the default (L) becomes unreadable with a logo.
        let level = match query.level.as_deref() {
            Some(v) => parse_ec_level(v)?,
            None if will_render_logo => ErrorCorrectionLevel::H,
            None => ErrorCorrectionLevel::L,
        };
        let fg = parse_hex_color(query.fg_color.as_deref().unwrap_or("#000000"), "fgColor")?;
        let bg = parse_hex_color(query.bg_color.as_deref().unwrap_or("#FFFFFF"), "bgColor")?;
        let logo = if will_render_logo {
            Some(fetch_logo(query.logo.as_deref().unwrap()).await?)
        } else {
            None
        };

        let dot_type = match query.dot_type.as_deref() {
            Some(v) => parse_dot_type(v)?,
            None => DotType::Rounded,
        };
        let corner_square_type = match query.corner_square_type.as_deref() {
            Some(v) => parse_corner_square_type(v)?,
            None => CornerSquareType::ExtraRounded,
        };
        let corner_dot_type = match query.corner_dot_type.as_deref() {
            Some(v) => parse_corner_dot_type(v)?,
            None => CornerDotType::Dot,
        };
        let shape = match query.shape.as_deref() {
            Some(v) => parse_shape(v)?,
            None => ShapeType::Square,
        };
        let dot_color = match query.dot_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "dotColor")?),
            None => None,
        };
        let corner_square_color = match query.corner_square_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "cornerSquareColor")?),
            None => None,
        };
        let corner_dot_color = match query.corner_dot_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "cornerDotColor")?),
            None => None,
        };

        Ok(Self {
            size,
            margin,
            level,
            fg,
            bg,
            logo,
            dot_type,
            corner_square_type,
            corner_dot_type,
            shape,
            dot_color,
            corner_square_color,
            corner_dot_color,
        })
    }
}

fn parse_ec_level(value: &str) -> Result<ErrorCorrectionLevel, String> {
    match value {
        "L" | "l" => Ok(ErrorCorrectionLevel::L),
        "M" | "m" => Ok(ErrorCorrectionLevel::M),
        "Q" | "q" => Ok(ErrorCorrectionLevel::Q),
        "H" | "h" => Ok(ErrorCorrectionLevel::H),
        _ => Err("level must be one of L, M, Q, H".to_string()),
    }
}

fn parse_dot_type(value: &str) -> Result<DotType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(DotType::Square),
        "dots" => Ok(DotType::Dots),
        "rounded" => Ok(DotType::Rounded),
        "classy" => Ok(DotType::Classy),
        "classy-rounded" | "classyrounded" => Ok(DotType::ClassyRounded),
        "extra-rounded" | "extrarounded" => Ok(DotType::ExtraRounded),
        _ => Err(
            "dotType must be one of square, dots, rounded, classy, classy-rounded, extra-rounded"
                .to_string(),
        ),
    }
}

fn parse_corner_square_type(value: &str) -> Result<CornerSquareType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(CornerSquareType::Square),
        "dot" => Ok(CornerSquareType::Dot),
        "extra-rounded" | "extrarounded" => Ok(CornerSquareType::ExtraRounded),
        _ => Err("cornerSquareType must be one of square, dot, extra-rounded".to_string()),
    }
}

fn parse_corner_dot_type(value: &str) -> Result<CornerDotType, String> {
    match value.to_ascii_lowercase().as_str() {
        "dot" => Ok(CornerDotType::Dot),
        "square" => Ok(CornerDotType::Square),
        _ => Err("cornerDotType must be one of dot, square".to_string()),
    }
}

fn parse_shape(value: &str) -> Result<ShapeType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(ShapeType::Square),
        "circle" => Ok(ShapeType::Circle),
        _ => Err("shape must be one of square, circle".to_string()),
    }
}

fn parse_hex_color(value: &str, name: &str) -> Result<Color, String> {
    let value = value.trim();
    let Some(hex) = value.strip_prefix('#') else {
        return Err(format!("{name} must be #RGB or #RRGGBB"));
    };
    if hex.len() != 3 && hex.len() != 6 {
        return Err(format!("{name} must be #RGB or #RRGGBB"));
    }
    Color::from_hex(value).map_err(|_| format!("{name} must be #RGB or #RRGGBB"))
}

async fn fetch_logo(url: &str) -> Result<LogoImage, String> {
    crate::core::validation::validate_web_url(url).map_err(|e| format!("logo: {e}"))?;
    // SSRF guard: validate_web_url only checks the user-supplied string. Following redirects
    // would let a public origin rebind to an internal IP mid-request, so disable them outright.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(QR_LOGO_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("logo client error: {e}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("logo fetch failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("logo fetch returned {}", response.status()));
    }
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        let content_type = content_type.to_str().unwrap_or_default().to_lowercase();
        let allowed = content_type.starts_with("image/png")
            || content_type.starts_with("image/jpeg")
            || content_type.starts_with("image/webp");
        if !allowed {
            return Err("logo must be PNG, JPEG, or WebP".to_string());
        }
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("logo read failed: {e}"))?;
    if bytes.len() > QR_MAX_LOGO_BYTES {
        return Err(format!("logo must be under {QR_MAX_LOGO_BYTES} bytes"));
    }

    let image = image::load_from_memory(&bytes)
        .map_err(|_| "logo must be a valid PNG, JPEG, or WebP image".to_string())?;
    let mut png_bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| format!("logo encode failed: {e}"))?;

    Ok(LogoImage { png_bytes })
}

fn render_qr(
    url: &str,
    options: &QrRenderOptions,
    format: QrOutputFormat,
) -> Result<Vec<u8>, String> {
    let margin_px = options.margin.saturating_mul(10);
    let dot_color = options.dot_color.unwrap_or(options.fg);
    let corner_square_color = options.corner_square_color.unwrap_or(options.fg);
    let corner_dot_color = options.corner_dot_color.unwrap_or(options.fg);
    let mut builder = QRCodeStyling::builder()
        .data(url)
        .size(options.size)
        .margin(margin_px)
        .shape(options.shape)
        .qr_options(QROptions::new().with_error_correction_level(options.level))
        .dots_options(DotsOptions::new(options.dot_type).with_color(dot_color))
        .corners_square_options(
            CornersSquareOptions::new(options.corner_square_type).with_color(corner_square_color),
        )
        .corners_dot_options(
            CornersDotOptions::new(options.corner_dot_type).with_color(corner_dot_color),
        )
        .background_options(BackgroundOptions::new(options.bg));

    if let Some(logo) = &options.logo {
        builder = builder.image(logo.png_bytes.clone()).image_options(
            ImageOptions::new()
                .with_image_size(0.22)
                .with_margin(6)
                .with_hide_background_dots(true)
                .with_save_as_blob(true),
        );
    }

    let qr = builder
        .build()
        .map_err(|e| format!("failed to build QR code: {e}"))?;
    match format {
        QrOutputFormat::Png => qr
            .render(OutputFormat::Png)
            .map_err(|e| format!("failed to render PNG: {e}")),
        QrOutputFormat::Svg => qr
            .render(OutputFormat::Svg)
            .map_err(|e| format!("failed to render SVG: {e}")),
    }
}

// ── Helpers ──

fn is_valid_link_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
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
    match domains
        .iter()
        .find(|d| d.verified && d.role == DomainRole::Primary)
        .or_else(|| domains.iter().find(|d| d.verified))
    {
        Some(d) => (Some(d.domain.clone()), true),
        None => (domains.first().map(|d| d.domain.clone()), false),
    }
}

async fn canonical_link_url(state: &AppState, link: &Link) -> String {
    let domain = crate::services::links::service::resolve_verified_primary_domain(
        state.domains_repo.as_deref(),
        &link.tenant_id,
    )
    .await;
    crate::services::links::service::build_canonical_link_url(
        &state.config.public_url,
        &link.link_id,
        domain.as_deref(),
    )
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
