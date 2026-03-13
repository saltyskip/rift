use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use chrono::{Datelike, Utc};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use x402_axum::paygate::PaygateProtocol;
use x402_types::proto::v1;

use super::keys;
use super::repo::{self, AuthRepo, UsageDoc};
use crate::api::AppState;

/// Tenant identity injected by the auth middleware.
/// Handlers extract this via `Extension<TenantId>`.
#[derive(Debug, Clone)]
pub struct TenantId(pub ObjectId);

/// Auth + rate-limit middleware for protected endpoints.
///
/// Priority:
/// 1. API key present -> validate, check monthly quota, inject TenantId, proceed
/// 2. x402 payment header -> verify with facilitator, proceed, settle after
/// 3. No key, within IP daily limit -> proceed (anonymous free tier)
/// 4. No key, IP limit exceeded -> 429
pub async fn auth_gate(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_repo = match &state.auth_repo {
        Some(r) => r,
        None => return next.run(req).await,
    };

    let ip = client_ip(&req);
    let endpoint = req.uri().path().to_string();

    // ── Path 1: API key ──
    if let Some(raw_key) = extract_bearer(&req) {
        let key_id = match validate_api_key(auth_repo, &raw_key).await {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        // Inject tenant identity for downstream handlers.
        req.extensions_mut().insert(TenantId(key_id));

        let response = next.run(req).await;
        if response.status().is_success() {
            record_usage(auth_repo, Some(key_id), ip, &endpoint).await;
        }
        return response;
    }

    // ── Path 2: x402 payment header ──
    let payment_header = state
        .config
        .x402_enabled
        .then(|| req.headers().get("x-payment").cloned())
        .flatten();

    if let Some(header_val) = payment_header {
        let facilitator = match &state.facilitator {
            Some(f) => f,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({ "error": "Payments not configured", "code": "x402_disabled" })),
                )
                    .into_response();
            }
        };

        let verify_request = match decode_payment_header(
            header_val.as_bytes(),
            &state.x402_price_tags,
            &state.config.public_url,
            &endpoint,
            &state.config.x402_description,
        ) {
            Ok(vr) => vr,
            Err(resp) => return resp,
        };

        let verify_response = match facilitator.verify(&verify_request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Facilitator verify error: {e:?}");
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("Payment facilitator error: {e}"), "code": "facilitator_error" })),
                )
                    .into_response();
            }
        };

        if let Err(e) = v1::PriceTag::validate_verify_response(verify_response) {
            return (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({ "error": e.to_string(), "code": "payment_invalid" })),
            )
                .into_response();
        }

        let response = next.run(req).await;
        if response.status().is_success() {
            if let Err(e) = facilitator.settle(&verify_request).await {
                tracing::error!("x402 settlement failed: {e}");
            }
            record_usage(auth_repo, None, ip, &endpoint).await;
        }
        return response;
    }

    // ── Path 3: Anonymous / IP rate limit ──
    if let Err(resp) = check_anonymous_limit(&state, auth_repo, &ip).await {
        return resp;
    }

    let response = next.run(req).await;
    if response.status().is_success() {
        record_usage(auth_repo, None, ip, &endpoint).await;
    }
    response
}

// ── Helpers ──

fn client_ip(req: &Request) -> String {
    let from_header = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    from_header.unwrap_or_else(|| {
        req.extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    })
}

fn extract_bearer(req: &Request) -> Option<String> {
    let header = req.headers().get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    token.starts_with("rl_").then(|| token.to_string())
}

async fn record_usage(auth_repo: &AuthRepo, api_key_id: Option<ObjectId>, ip: String, endpoint: &str) {
    auth_repo
        .record_usage(UsageDoc {
            id: None,
            api_key_id,
            ip,
            endpoint: endpoint.to_string(),
            ts: repo::now_bson(),
        })
        .await;
}

async fn validate_api_key(auth_repo: &AuthRepo, raw_key: &str) -> Result<ObjectId, Response> {
    let hash = keys::hash_key(raw_key);
    let key_doc = auth_repo.find_key_by_hash(&hash).await.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid or unverified API key",
                "code": "invalid_key"
            })),
        )
            .into_response()
    })?;

    let key_id = key_doc.id.unwrap_or_else(ObjectId::new);
    let month_start = Utc::now()
        .date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let used = auth_repo.count_key_usage_since(&key_id, month_start).await;

    if used >= key_doc.monthly_quota {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Monthly quota exceeded",
                "code": "quota_exceeded",
                "used": used,
                "quota": key_doc.monthly_quota,
                "hint": "Contact us for higher limits"
            })),
        )
            .into_response());
    }

    Ok(key_id)
}

fn decode_payment_header(
    header_bytes: &[u8],
    price_tags: &[v1::PriceTag],
    public_url: &str,
    endpoint: &str,
    description: &str,
) -> Result<x402_types::proto::VerifyRequest, Response> {
    let decoded = x402_types::util::Base64Bytes::from(header_bytes)
        .decode()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid payment header encoding", "code": "bad_payment" })),
            )
                .into_response()
        })?;

    let payment_payload: v1::PaymentPayload =
        serde_json::from_slice(decoded.as_ref()).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Malformed payment payload", "code": "bad_payment" })),
            )
                .into_response()
        })?;

    let resource = x402_types::proto::v2::ResourceInfo {
        description: Some(description.to_string()),
        mime_type: Some("application/json".to_string()),
        url: format!("{public_url}{endpoint}"),
    };

    v1::PriceTag::make_verify_request(payment_payload, price_tags, &resource).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string(), "code": "payment_verification_failed" })),
        )
            .into_response()
    })
}

async fn check_anonymous_limit(
    state: &AppState,
    auth_repo: &AuthRepo,
    ip: &str,
) -> Result<(), Response> {
    let today_start = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let ip_used = auth_repo.count_ip_usage_since(ip, today_start).await;
    let daily_limit = state.config.free_daily_limit;

    if ip_used < daily_limit {
        return Ok(());
    }

    Err((
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "Daily free limit exceeded",
            "code": "rate_limited",
            "used": ip_used,
            "limit": daily_limit,
            "hint": "Sign up for a free API key (100/month) at POST /v1/auth/signup"
        })),
    )
        .into_response())
}
