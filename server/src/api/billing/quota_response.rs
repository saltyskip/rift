//! Shared mapping from `QuotaError` to an HTTP response. Every enforcement
//! site in the API calls `QuotaService::check(...).or_else(to_response)` so
//! the 402 shape stays consistent across link-create, domain-create, webhook
//! create, team invite, and the event-tracking hot paths.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

use crate::services::billing::models::BillingError;
use crate::services::billing::quota::QuotaError;

/// Turn a `QuotaError` into the documented error response.
///
/// - `Exceeded` → `402 Payment Required` with the upgrade-context fields.
/// - `Billing(TenantNotFound)` → `404` (the authenticated tenant id resolved
///   nowhere — unusual, probably tampering).
/// - `Billing(Internal)` → `500 db_error`.
pub fn to_response(err: QuotaError) -> Response {
    match err {
        QuotaError::Exceeded {
            resource,
            limit,
            current,
        } => (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": format!(
                    "Quota exceeded on {} ({}/{}). Upgrade to continue.",
                    resource.code(),
                    current,
                    limit
                ),
                "code": "quota_exceeded",
                "resource": resource.code(),
                "limit": limit,
                "current": current,
                "upgrade_url": "/pricing",
            })),
        )
            .into_response(),
        QuotaError::Billing(BillingError::TenantNotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Tenant not found", "code": "tenant_not_found" })),
        )
            .into_response(),
        QuotaError::Billing(BillingError::Internal(e)) => {
            tracing::error!(error = %e, "quota_check_db_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}
