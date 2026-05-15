//! SDK-authenticated lifecycle endpoints.
//!
//! Three verbs of the growth funnel — `click`, `attribute`, `identify` —
//! plus the matching webhooks. The data lives in the `links` service
//! (`Install` + `AttributionEvent` documents), but the `/v1/lifecycle/*`
//! URL surface and the `pk_live_` SDK auth gate make this its own
//! transport slice.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::models::{SdkDomain, TenantId};
use crate::api::links::routes::{check_link_resolvable, detect_platform, Platform};
use crate::app::AppState;
use crate::core::webhook_dispatcher::{
    AttributeEventPayload, ClickEventPayload, IdentifyEventPayload,
};
use crate::services::links::models::{
    AttributeOutcome, AttributeRequest, ClickRequest, IdentifyOutcome, IdentifyRequest, Install,
    Link,
};

// ── POST /v1/lifecycle/click — SDK-authenticated click ──

#[utoipa::path(
    post,
    path = "/v1/lifecycle/click",
    tag = "Lifecycle",
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
pub async fn lifecycle_click(
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

// ── POST /v1/lifecycle/attribute — Attribute a link touch to an install ──

#[utoipa::path(
    post,
    path = "/v1/lifecycle/attribute",
    tag = "Lifecycle",
    request_body = AttributeRequest,
    responses(
        (status = 200, description = "Attribution event recorded"),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Link not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn lifecycle_attribute(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<AttributeRequest>,
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

    let Some(svc) = &state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Service not configured", "code": "no_service" })),
        )
            .into_response();
    };

    let outcome = match svc
        .record_attribute_event(
            link.tenant_id,
            &req.link_id,
            &req.install_id,
            &req.app_version,
        )
        .await
    {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("Failed to record attribute event: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    // Every `attribute` call fires the webhook. `user_id` is populated when
    // the install was already identified (existing-install re-attribution
    // path) — that's how downstream subscribers credit a teammate who
    // clicks a campaign link without reinstalling.
    if let Some(dispatcher) = &state.webhook_dispatcher {
        let install = match &outcome {
            AttributeOutcome::FirstTouch(i) | AttributeOutcome::Retouch(i) => i,
        };
        let link_metadata = link
            .metadata
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        dispatcher.dispatch_attribute(AttributeEventPayload {
            tenant_id: link.tenant_id.to_hex(),
            link_id: req.link_id.clone(),
            install_id: req.install_id.clone(),
            app_version: req.app_version.clone(),
            user_id: install.user_id.clone(),
            link_metadata,
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    Json(json!({ "success": true })).into_response()
}

// ── PUT /v1/lifecycle/identify — Bind install to user (SDK-authenticated) ──
//
// SDK-authenticated (pk_live_) because the install_id is opaque and only
// lives in the mobile SDK; no flow produces the inputs a backend would need
// to call this endpoint with a secret key.

#[utoipa::path(
    put,
    path = "/v1/lifecycle/identify",
    tag = "Lifecycle",
    request_body = IdentifyRequest,
    responses(
        (status = 200, description = "Install bound to user"),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "No install found for this install_id", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn lifecycle_identify(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<IdentifyRequest>,
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

    let outcome = repo
        .identify_install(&tenant.0, &req.install_id, &req.user_id)
        .await;

    match outcome {
        Ok(IdentifyOutcome::NewBind(install)) => {
            // First-time bind for this install — fire the `identify`
            // webhook so subscribers can react (credit the user for a
            // welcome bonus campaign, etc.). Failures inside the fire-site
            // are non-fatal: the bind already committed; webhook dispatch
            // is best-effort.
            fire_identify_event(&state, install).await;
            Json(json!({ "success": true })).into_response()
        }
        Ok(IdentifyOutcome::AlreadyBound(_)) => {
            // Idempotent replay — install was already bound to this same
            // user_id. Return 200 (desired end state holds) but
            // intentionally skip the webhook so subscribers don't
            // double-fulfill credits / entitlements on SDK auto-retry.
            tracing::debug!(
                install_id = %req.install_id,
                user_id = %req.user_id,
                "identify already bound to this user; skipping webhook"
            );
            Json(json!({ "success": true })).into_response()
        }
        Ok(IdentifyOutcome::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No install found for this install_id", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to identify install: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

/// Load the link, then dispatch the `identify` webhook with the full
/// `{user_id, link_id, link_metadata}` triple. The bind has already
/// committed, so any anomaly here (missing link, etc.) is a soft warning
/// rather than an error path.
async fn fire_identify_event(state: &Arc<AppState>, install: Install) {
    let (Some(repo), Some(dispatcher)) = (&state.links_repo, &state.webhook_dispatcher) else {
        return;
    };

    let link: Link = match repo
        .find_link_by_tenant_and_id(&install.tenant_id, &install.first_link_id)
        .await
    {
        Ok(Some(l)) => l,
        Ok(None) => {
            tracing::warn!(
                link_id = %install.first_link_id,
                "identify webhook: link missing for install"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, "identify webhook: failed to load link");
            return;
        }
    };

    let link_metadata = link
        .metadata
        .as_ref()
        .and_then(|d| serde_json::to_value(d).ok());

    // `user_id` is always `Some` here — `IdentifyOutcome::NewBind` is
    // constructed post-update with `user_id` set. Defensive default keeps
    // the contract local rather than panicking on a future refactor.
    let user_id = install.user_id.clone().unwrap_or_default();

    dispatcher.dispatch_identify(IdentifyEventPayload {
        tenant_id: install.tenant_id.to_hex(),
        user_id,
        link_id: install.first_link_id,
        install_id: install.install_id,
        link_metadata,
        timestamp: Utc::now().to_rfc3339(),
    });
}
