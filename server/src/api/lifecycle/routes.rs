//! SDK-authenticated lifecycle endpoints.
//!
//! Three verbs of the growth funnel — `click`, `attribute`, `identify` —
//! plus the matching webhooks. Data lives in the `links` service
//! (`attribution_events`) and the `app_users` service (identity); the
//! `/v1/lifecycle/*` URL surface and the `pk_live_` SDK auth gate make
//! this its own transport slice.

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
    AttributeRequest, ClickRequest, CreditedLinks, IdentifyOutcome, IdentifyRequest, LinkError,
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

    let user_id = match svc
        .record_attribute_event(
            link.tenant_id,
            &req.link_id,
            &req.install_id,
            &req.app_version,
            req.context.clone().map(Into::into),
        )
        .await
    {
        Ok(uid) => uid,
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
    // the install was already identified (resolved by the service via
    // `app_users`) — that's how downstream subscribers credit a teammate
    // who clicks a campaign link without reinstalling.
    if let Some(dispatcher) = &state.webhook_dispatcher {
        let link_metadata = link
            .metadata
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        dispatcher.dispatch_attribute(AttributeEventPayload {
            tenant_id: link.tenant_id.to_hex(),
            link_id: req.link_id.clone(),
            install_id: req.install_id.clone(),
            app_version: req.app_version.clone(),
            user_id,
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
        (status = 409, description = "Install already bound to a different user", body = crate::error::ErrorResponse),
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

    let Some(svc) = &state.links_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc
        .identify_install(&tenant.0, &req.install_id, &req.user_id)
        .await
    {
        Ok(IdentifyOutcome::Created(credited)) | Ok(IdentifyOutcome::InstallAdded(credited)) => {
            // Real state change — fire the `identify` webhook so
            // subscribers can react (welcome bonus, etc.). Best-effort.
            tracing::debug!(
                install_id = %req.install_id,
                user_id = %req.user_id,
                "identify bound; firing webhook"
            );
            fire_identify_event(&state, &tenant.0, &req.install_id, &req.user_id, credited);
            Json(json!({ "success": true })).into_response()
        }
        Ok(IdentifyOutcome::AlreadyPresent) => {
            // Idempotent replay — return 200 but skip the webhook so
            // subscribers don't double-fulfill on SDK auto-retry.
            tracing::debug!(
                install_id = %req.install_id,
                user_id = %req.user_id,
                "identify already present; skipping webhook"
            );
            Json(json!({ "success": true })).into_response()
        }
        Err(LinkError::IdentifyConflict { existing_user_id }) => {
            tracing::info!(
                install_id = %req.install_id,
                attempted_user_id = %req.user_id,
                existing_user_id = %existing_user_id,
                "identify rejected: install already bound to a different user"
            );
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "install_id is already bound to a different user",
                    "code": "identify_conflict",
                })),
            )
                .into_response()
        }
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

/// Dispatch the `identify` webhook. Carries `first_touch_link_id` and
/// `last_touch_link_id` computed server-side from the user's unified
/// attribution chain (post-backfill) so receivers can act on the
/// acquisition source without querying Rift back.
fn fire_identify_event(
    state: &Arc<AppState>,
    tenant_id: &mongodb::bson::oid::ObjectId,
    install_id: &str,
    user_id: &str,
    credited: CreditedLinks,
) {
    let Some(dispatcher) = &state.webhook_dispatcher else {
        return;
    };
    dispatcher.dispatch_identify(IdentifyEventPayload {
        tenant_id: tenant_id.to_hex(),
        user_id: user_id.to_string(),
        install_id: install_id.to_string(),
        first_touch_link_id: credited.first_touch_link_id,
        first_touch_link_metadata: credited.first_touch_link_metadata,
        last_touch_link_id: credited.last_touch_link_id,
        last_touch_link_metadata: credited.last_touch_link_metadata,
        timestamp: Utc::now().to_rfc3339(),
    });
}
