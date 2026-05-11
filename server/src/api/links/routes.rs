use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde_json::json;
use std::sync::Arc;

use super::landing::{render_smart_landing_page, LandingPageContext};
use super::models::{QrCodeQuery, ResolveQuery};
use super::qr::{render_link_qr, QrOutputFormat};
use crate::api::auth::models::{CallerScope, TenantId};
use crate::app::AppState;
use crate::core::webhook_dispatcher::ClickEventPayload;
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;
use crate::services::links::models::LinkError;
use crate::services::links::models::*;

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
        Err(crate::services::links::models::LinkError::NotFound) => (
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
/// `pub(crate)` so the attribution slice (`api/attribution/routes.rs`)
/// can reuse the same gate without duplicating the policy.
pub(crate) fn check_link_resolvable(link: &Link) -> Option<Response> {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Platform {
    Ios,
    Android,
    Other,
}

impl Platform {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Other => "other",
        }
    }
}

pub(crate) fn detect_platform(user_agent: &str) -> Platform {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        Platform::Ios
    } else if ua.contains("android") {
        Platform::Android
    } else {
        Platform::Other
    }
}

fn is_valid_link_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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

pub(crate) async fn canonical_link_url(state: &AppState, link: &Link) -> String {
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

pub(crate) fn urlencoding(s: &str) -> String {
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
