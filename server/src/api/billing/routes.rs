use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::middleware::TenantId;
use crate::app::AppState;
use crate::services::auth::tenants::repo::{BillingMethod, PlanTier, SubscriptionStatus};
use crate::services::billing::limits::limits_for;
use crate::services::billing::models::{BillingError, BillingStatus};

/// Slim JSON shape for the status endpoint. Rendered from `BillingStatus` so
/// the external contract stays stable if we add internal fields later.
#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingStatusResponse {
    pub plan_tier: PlanTier,
    pub effective_tier: PlanTier,
    pub comp_active: bool,
    pub billing_method: BillingMethod,
    pub status: SubscriptionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<i64>,
    pub limits: LimitsView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LimitsView {
    pub max_links: Option<u64>,
    pub max_events_per_month: Option<u64>,
    pub max_domains: Option<u64>,
    pub max_team_members: Option<u64>,
    pub max_webhooks: Option<u64>,
    pub analytics_retention: &'static str,
}

fn render(status: BillingStatus) -> BillingStatusResponse {
    let limits = limits_for(status.effective_tier);
    BillingStatusResponse {
        plan_tier: status.plan_tier,
        effective_tier: status.effective_tier,
        comp_active: status.comp_active,
        billing_method: status.billing_method,
        status: status.status,
        current_period_end: status.current_period_end.map(|d| d.timestamp_millis()),
        limits: LimitsView {
            max_links: limits.max_links,
            max_events_per_month: limits.max_events_per_month,
            max_domains: limits.max_domains,
            max_team_members: limits.max_team_members,
            max_webhooks: limits.max_webhooks,
            analytics_retention: limits.retention_bucket,
        },
    }
}

#[utoipa::path(
    get,
    path = "/v1/billing/status",
    tag = "Billing",
    responses(
        (status = 200, description = "Current billing state for the authenticated tenant", body = BillingStatusResponse),
        (status = 401, description = "Missing or invalid API key", body = crate::error::ErrorResponse),
        (status = 503, description = "Billing service not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_billing_status(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(service) = state.billing_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Billing service not configured", "code": "no_billing" })),
        )
            .into_response();
    };

    match service.status(&tenant.0).await {
        Ok(status) => (StatusCode::OK, Json(render(status))).into_response(),
        Err(BillingError::TenantNotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Tenant not found", "code": "tenant_not_found" })),
        )
            .into_response(),
        Err(BillingError::Internal(e)) => {
            tracing::error!(error = %e, "billing_status_db_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}
