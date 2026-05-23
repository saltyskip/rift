use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use mongodb::bson::DateTime as BsonDateTime;
use serde_json::json;
use std::sync::Arc;

use super::models::StatsQuery;
use crate::app::AppState;
use crate::services::analytics::models::{AnalyticsError, FunnelParams, FunnelResult};
use crate::services::auth::permissions::AuthContext;
use crate::services::links::models::CreditModel;

const DEFAULT_LOOKBACK_DAYS: i64 = 30;

// ── GET /v1/analytics/stats ──

#[utoipa::path(
    get,
    path = "/v1/analytics/stats",
    tag = "Analytics",
    params(StatsQuery),
    responses(
        (status = 200, description = "Branched funnel response across the link_ids set", body = FunnelResult),
        (status = 400, description = "Invalid query", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 503, description = "Database not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Query(q): Query<StatsQuery>,
) -> Response {
    let Some(svc) = &state.analytics_service else {
        return error_response(AnalyticsError::Unavailable, StatusCode::SERVICE_UNAVAILABLE);
    };

    let params = match parse_params(q) {
        Ok(p) => p,
        Err(e) => return error_response(e, StatusCode::BAD_REQUEST),
    };

    match svc.funnel(&ctx, params).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => error_response_for(e),
    }
}

// ── Helpers (transport: query parsing, error → HTTP mapping) ──

fn parse_params(q: StatsQuery) -> Result<FunnelParams, AnalyticsError> {
    let link_ids: Vec<String> = q
        .link_ids
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_string())
        })
        .collect();

    let to = match q.to.as_deref() {
        Some(s) => parse_iso(s)
            .ok_or_else(|| AnalyticsError::Internal("to must be ISO 8601".to_string()))?,
        None => Utc::now(),
    };
    let from = match q.from.as_deref() {
        Some(s) => parse_iso(s)
            .ok_or_else(|| AnalyticsError::Internal("from must be ISO 8601".to_string()))?,
        None => to - chrono::Duration::days(DEFAULT_LOOKBACK_DAYS),
    };

    Ok(FunnelParams {
        link_ids,
        from: BsonDateTime::from_millis(from.timestamp_millis()),
        to: BsonDateTime::from_millis(to.timestamp_millis()),
        credit: CreditModel::parse(q.credit.as_deref()),
    })
}

fn parse_iso(s: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn error_response_for(e: AnalyticsError) -> Response {
    if let AnalyticsError::Forbidden(authz) = e {
        return crate::api::auth::forbidden_response::to_response(authz);
    }
    let status = match &e {
        AnalyticsError::EmptyLinkIds | AnalyticsError::InvalidDateRange => StatusCode::BAD_REQUEST,
        AnalyticsError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        AnalyticsError::Forbidden(_) => unreachable!(),
        AnalyticsError::Internal(msg) => {
            tracing::error!("analytics query failed: {msg}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    error_response(e, status)
}

fn error_response(e: AnalyticsError, status: StatusCode) -> Response {
    (
        status,
        Json(json!({ "error": e.to_string(), "code": e.code() })),
    )
        .into_response()
}
