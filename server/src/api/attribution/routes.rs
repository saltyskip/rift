//! SDK-authenticated attribution endpoints.
//!
//! The attribution data domain itself is part of the links service
//! (`Attribution` documents live alongside `Link` in `services/links/`),
//! but the `/v1/attribution/*` URL surface and the `pk_live_` SDK auth
//! gate make this its own transport slice.

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
    AttributionEventPayload, ClickEventPayload, IdentifyEventPayload,
};
use crate::services::links::models::{
    AttributionReportRequest, AttributionResponse, ClickRequest, LinkAttributionRequest,
};

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
        Ok(true) => {
            // Fire the `identify` webhook so receivers can react to the
            // newly-bound (user_id, link_id) pair (e.g. credit the user for
            // a welcome bonus campaign). Failures here are non-fatal — the
            // attribution bind already succeeded and is the load-bearing
            // outcome; webhook dispatch is best-effort.
            fire_identify_event(&state, &tenant.0, &req.install_id, &req.user_id).await;
            Json(json!({ "success": true })).into_response()
        }
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

/// Resolve the attribution + link, then dispatch the `identify` webhook with
/// the full `{user_id, link_id, link_metadata}` triple. Logs and returns on
/// any anomaly — the attribution write has already committed, so a missing
/// link or absent metadata is a soft warning, not an error path.
async fn fire_identify_event(
    state: &Arc<AppState>,
    tenant_id: &mongodb::bson::oid::ObjectId,
    install_id: &str,
    user_id: &str,
) {
    let (Some(repo), Some(dispatcher)) = (&state.links_repo, &state.webhook_dispatcher) else {
        return;
    };

    let attribution = match repo.find_attribution_by_user(tenant_id, user_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            tracing::warn!(
                user_id,
                install_id,
                "identify webhook: attribution vanished post-bind"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, "identify webhook: failed to look up attribution");
            return;
        }
    };

    let link = match repo
        .find_link_by_tenant_and_id(tenant_id, &attribution.link_id)
        .await
    {
        Ok(Some(l)) => l,
        Ok(None) => {
            tracing::warn!(
                link_id = %attribution.link_id,
                "identify webhook: link missing for attribution"
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

    dispatcher.dispatch_identify(IdentifyEventPayload {
        tenant_id: tenant_id.to_hex(),
        user_id: user_id.to_string(),
        link_id: attribution.link_id,
        install_id: install_id.to_string(),
        link_metadata,
        timestamp: Utc::now().to_rfc3339(),
    });
}
