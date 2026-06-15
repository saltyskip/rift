use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use crate::app::AppState;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::tenants::models::TenantError;
use crate::services::landing::models::LandingTheme;

// ── Handlers ──

#[utoipa::path(
    get,
    path = "/v1/tenant/branding",
    tag = "Tenant",
    responses(
        (status = 200, description = "Current landing-page branding (Rift defaults if unset)", body = LandingTheme),
        (status = 503, description = "Database not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn get_branding(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Response {
    let Some(svc) = &state.tenants_service else {
        return no_database();
    };
    match svc.get_landing_theme(&ctx.tenant_id).await {
        Ok(theme) => (StatusCode::OK, Json(theme)).into_response(),
        Err(e) => error_response(e),
    }
}

#[utoipa::path(
    put,
    path = "/v1/tenant/branding",
    tag = "Tenant",
    request_body = LandingTheme,
    responses(
        (status = 200, description = "Branding saved", body = LandingTheme),
        (status = 400, description = "Invalid branding", body = crate::error::ErrorResponse),
        (status = 503, description = "Database not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx, theme))]
pub async fn update_branding(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(theme): Json<LandingTheme>,
) -> Response {
    let Some(svc) = &state.tenants_service else {
        return no_database();
    };
    match svc
        .update_landing_theme(&ctx.tenant_id, theme.clone())
        .await
    {
        Ok(()) => {
            tracing::info!(tenant_id = %ctx.tenant_id, "Landing branding updated");
            (StatusCode::OK, Json(theme)).into_response()
        }
        Err(e) => error_response(e),
    }
}

// ── Helpers ──

fn no_database() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Database not configured", "code": "no_database" })),
    )
        .into_response()
}

fn error_response(e: TenantError) -> Response {
    let (status, code) = match e {
        TenantError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_branding"),
        TenantError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
    };
    (
        status,
        Json(json!({ "error": e.to_string(), "code": code })),
    )
        .into_response()
}
