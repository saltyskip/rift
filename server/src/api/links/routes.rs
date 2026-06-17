use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use mongodb::bson::DateTime;
use serde_json::json;
use std::sync::Arc;

use super::landing::{render_smart_landing_page, LandingPageContext};
use super::models::{QrCodeQuery, ResolveQuery};
use super::qr::{render_link_qr, QrOutputFormat};
use crate::api::auth::models::TenantId;
use crate::app::AppState;
use crate::core::platform::{detect_os, Os};
use crate::core::webhook_dispatcher::ClickEventPayload;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::tenants::models::RedirectMode;
use crate::services::auth::tenants::repo::TenantsRepository;
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;
use crate::services::landing::models::LandingTheme;
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
#[tracing::instrument(skip(state, ctx, req))]
pub async fn create_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
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
    match svc.create_link(&ctx, req).await {
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
#[tracing::instrument(skip(state, ctx, req))]
pub async fn create_links_bulk(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(req): Json<BulkCreateLinksRequest>,
) -> Response {
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.create_links_bulk(&ctx, req).await {
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
#[tracing::instrument(skip(state, ctx))]
pub async fn list_links(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Query(query): Query<ListLinksQuery>,
) -> Response {
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.list_links(&ctx, query.limit, query.cursor).await {
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
#[tracing::instrument(skip(state, ctx))]
pub async fn get_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(link_id): Path<String>,
) -> Response {
    let Some(svc) = &state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.get_link(&ctx, &link_id).await {
        Ok(detail) => Json(json!(detail)).into_response(),
        Err(LinkError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response(),
        Err(LinkError::Forbidden(e)) => crate::api::auth::forbidden_response::to_response(e),
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
#[tracing::instrument(skip(state, ctx, req))]
pub async fn update_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
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

    match svc.update_link(&ctx, &link_id, req).await {
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
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_link(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(link_id): Path<String>,
) -> Response {
    let Some(ref svc) = state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.delete_link(&ctx, &link_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => link_error_to_response(e),
    }
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

        return match links_svc
            .resolve_alternate(&domain.tenant_id, &link_id, detect_os(&headers))
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

    let os = detect_os(headers);

    // Quota check + retention bucket + DB write all happen inside the
    // service method so MCP / CLI / any future transport that records a
    // click can't bypass enforcement.
    if let Some(ref svc) = state.links_service {
        svc.record_click(
            link.tenant_id,
            link_id,
            user_agent.clone(),
            referer.clone(),
            Some(os.as_str().to_string()),
        )
        .await;
    }

    if let Some(dispatcher) = &state.webhook_dispatcher {
        dispatcher.dispatch_click(ClickEventPayload {
            tenant_id: link.tenant_id.to_string(),
            link_id: link_id.to_string(),
            user_agent,
            referer,
            platform: os.as_str().to_string(),
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
            "macos_store_url": link.macos_store_url,
            "windows_store_url": link.windows_store_url,
            "metadata": metadata,
            "agent_context": link.agent_context,
            "social_preview": link.social_preview,
            "_rift_meta": rift_meta,
        }))
        .into_response();
    }

    // `?redirect=1` skips the landing page and goes straight to the platform
    // destination — the SDK `Rift.click()` flow (which has already stamped the
    // clipboard with a user gesture at the source). On iOS the App Store URL
    // opens the app if installed, else the store; desktop goes to web. Distinct
    // from `redirect_mode = Auto`, which is gated on a `Sec-Fetch-User` signal.
    if redirect {
        if let Some(url) = explicit_redirect_target(&link, os, link_id) {
            return Redirect::temporary(&url).into_response();
        }
        // No destination for this OS — fall through to landing page.
    }

    // Auto-redirect (redirect_mode = Auto): a zero-flash 307 for desktop Tier-1
    // targets, but only with a strong human-activation signal (Sec-Fetch-User:
    // ?1). Crawlers (no signal, no JS), mobile, Tier-2 clipboard stores, and
    // no-signal humans all fall through to the landing page — which keeps OG
    // unfurls intact and drives the Universal-Link/clipboard tap.
    if link.redirect_mode.unwrap_or(RedirectMode::Off) == RedirectMode::Auto
        && has_user_activation(headers)
    {
        if let Some(url) = auto_redirect_target(&link, os, link_id) {
            return auto_redirect_response(&url);
        }
    }

    // If this link has per-platform destinations, serve the smart landing page.
    let has_platform_destinations = link.ios_deep_link.is_some()
        || link.android_deep_link.is_some()
        || link.ios_store_url.is_some()
        || link.android_store_url.is_some()
        || link.macos_store_url.is_some()
        || link.windows_store_url.is_some();

    // Compute link status and tenant domain for landing page and JSON-LD.
    let link_status = compute_link_status(&link);
    let (tenant_domain, tenant_verified) =
        lookup_tenant_domain(state.domains_repo.as_deref(), &link.tenant_id).await;

    if has_platform_destinations {
        // Brand config: the tenant's stored LandingTheme, else Rift defaults,
        // then per-link overrides merged on top. (Per-link content —
        // social_preview — is overlaid separately in the renderer.)
        let mut theme = resolve_landing_theme(state.tenants_repo.as_deref(), &link.tenant_id).await;
        if let Some(ov) = &link.landing_theme {
            theme = theme.merged_with(ov);
        }

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

        // Canonical URL of this link — encoded into the desktop QR.
        let link_url = canonical_link_url(state, &link).await;

        let html = render_smart_landing_page(&LandingPageContext {
            os,
            link: &link,
            link_id,
            link_url: &link_url,
            theme: &theme,
            social_preview: link.social_preview.as_ref(),
            agent_context: link.agent_context.as_ref(),
            link_status,
            tenant_domain: tenant_domain.as_deref(),
            tenant_verified,
            alternate_domain: alternate_domain.as_deref(),
        });
        let mut resp = (StatusCode::OK, axum::response::Html(html)).into_response();
        // The landing HTML bakes in per-OS store buttons — never let a shared
        // cache cross-serve it across platforms.
        set_no_store_resolve_headers(&mut resp);
        return resp;
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
    if let LinkError::Forbidden(e) = err {
        return crate::api::auth::forbidden_response::to_response(e);
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
        | LinkError::InvalidLandingTheme(_)
        | LinkError::ThreatDetected(_)
        | LinkError::NoVerifiedDomain
        | LinkError::EmptyUpdate
        | LinkError::AffiliateScopeMismatch
        | LinkError::BatchTooLarge { .. }
        | LinkError::BatchEmpty
        | LinkError::BatchModeAmbiguous
        | LinkError::BatchModeMissing => StatusCode::BAD_REQUEST,
        LinkError::LinkIdTaken(_) | LinkError::IdentifyConflict { .. } => StatusCode::CONFLICT,
        LinkError::NotFound | LinkError::AffiliateNotFound => StatusCode::NOT_FOUND,
        LinkError::QuotaExceeded(_)
        | LinkError::Forbidden(_)
        | LinkError::BatchValidationFailed(_) => {
            unreachable!("handled above")
        }
        LinkError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = err.code();
    let message = err.to_string();
    (status, Json(json!({ "error": message, "code": code }))).into_response()
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

/// Resolve a tenant's landing-page branding, falling back to Rift defaults when
/// unset or unavailable. Always returns a usable theme so rendering never fails.
async fn resolve_landing_theme(
    tenants_repo: Option<&dyn TenantsRepository>,
    tenant_id: &crate::core::public_id::TenantId,
) -> LandingTheme {
    let Some(repo) = tenants_repo else {
        return LandingTheme::default();
    };
    repo.find_by_id(tenant_id)
        .await
        .ok()
        .flatten()
        .and_then(|t| t.landing_theme)
        .unwrap_or_default()
}

async fn lookup_tenant_domain(
    domains_repo: Option<&dyn DomainsRepository>,
    tenant_id: &crate::core::public_id::TenantId,
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

/// Append a query parameter, choosing `?` or `&` based on the existing URL.
/// The value is percent-encoded.
pub(crate) fn append_query_param(url: &str, key: &str, value: &str) -> String {
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}{key}={}", urlencoding(value))
}

/// Destination for the explicit `?redirect=1` path (the SDK `Rift.click()`
/// flow, which has already stamped the clipboard with a user gesture at the
/// source, so deferred attribution is handled before this redirect). Returns
/// the OS-appropriate store/web URL with its attribution param.
///
/// Intentionally returns a **bare** `web_url` on the desktop fallback (no
/// `rift_link`) to preserve historical behavior for callers that parse the
/// destination — this divergence from `auto_redirect_target` is deliberate, not
/// drift.
fn explicit_redirect_target(link: &Link, os: Os, link_id: &str) -> Option<String> {
    match os {
        Os::Ios => link.ios_store_url.clone(),
        Os::Android => link
            .android_store_url
            .as_deref()
            .map(|u| append_query_param(u, "referrer", &format!("rift_link={link_id}"))),
        Os::Mac => link
            .macos_store_url
            .clone()
            .or_else(|| link.web_url.clone()),
        Os::Windows => link
            .windows_store_url
            .as_deref()
            .map(|u| append_query_param(u, "cid", link_id))
            .or_else(|| link.web_url.clone()),
        Os::Other => link.web_url.clone(),
    }
}

/// True when the request carries a user-activation signal (`Sec-Fetch-User:
/// ?1`) — a real human navigation (click / address bar / QR open). Crawlers and
/// programmatic fetches don't set it, so it gates the zero-flash 307 without any
/// bot allowlist: anything without it falls through to the OG landing page.
fn has_user_activation(headers: &HeaderMap) -> bool {
    headers
        .get("sec-fetch-user")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim() == "?1")
        .unwrap_or(false)
}

/// Destination for the zero-flash auto-redirect (307), or `None` if the request
/// must fall through to the landing page. Tier-1 targets — where the identifier
/// rides the URL and no user gesture is needed — get the 307:
/// - **Android** → Play Store with `?referrer=rift_link` (the install referrer
///   survives the redirect). Installed users are intercepted by the App Link on
///   the associated domain *before* reaching Rift, so whoever hits the resolver
///   is effectively not-installed — there's no app-open to protect.
/// - **Windows** → Microsoft Store (`?cid`), else web (`?rift_link`).
///
/// **iOS** and **macOS** land instead: Apple stores carry no install referrer,
/// so deferred attribution depends on the clipboard, which needs a user-gesture
/// tap (the landing button). macOS also lands to let the page correct an iPad
/// (which reports as Mac) to the iOS App Store.
fn auto_redirect_target(link: &Link, os: Os, link_id: &str) -> Option<String> {
    match os {
        Os::Ios => None,
        Os::Android => link
            .android_store_url
            .as_deref()
            .map(|u| append_query_param(u, "referrer", &format!("rift_link={link_id}")))
            .or_else(|| web_redirect_target(link, link_id)),
        Os::Windows => link
            .windows_store_url
            .as_deref()
            .map(|u| append_query_param(u, "cid", link_id))
            .or_else(|| web_redirect_target(link, link_id)),
        // macOS always lands when a native or iOS target exists. Mac App Store
        // is clipboard-deferred (Tier-2), AND an iPad in desktop mode is
        // indistinguishable from a Mac by headers alone — landing lets the page
        // correct iPad → iOS App Store by touch detection. Only a pure web link
        // (no Mac store, no iOS target) is safe to 307.
        Os::Mac => {
            if link.macos_store_url.is_some()
                || link.ios_store_url.is_some()
                || link.ios_deep_link.is_some()
            {
                None
            } else {
                web_redirect_target(link, link_id)
            }
        }
        Os::Other => web_redirect_target(link, link_id),
    }
}

fn web_redirect_target(link: &Link, link_id: &str) -> Option<String> {
    link.web_url
        .as_deref()
        .map(|u| append_query_param(u, "rift_link", link_id))
}

/// Mark a resolve response uncacheable and OS/activation-varying. The same URL
/// returns a 307 (with `Sec-Fetch-User`) or the landing page (without), and the
/// destination/buttons vary by detected OS — a shared cache must not reuse one
/// visitor's response for another.
fn set_no_store_resolve_headers(resp: &mut Response) {
    let h = resp.headers_mut();
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(
        header::VARY,
        HeaderValue::from_static("Sec-Fetch-User, Sec-CH-UA-Platform, User-Agent"),
    );
}

fn auto_redirect_response(url: &str) -> Response {
    let mut resp = Redirect::temporary(url).into_response();
    set_no_store_resolve_headers(&mut resp);
    resp
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
