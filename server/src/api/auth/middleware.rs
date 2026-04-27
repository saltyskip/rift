use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use x402_axum::paygate::PaygateProtocol;
use x402_types::proto::v1;

use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::auth::usage::repo::{self as usage_repo};

/// Tenant identity injected by the auth middleware.
/// Handlers extract this via `Extension<TenantId>`.
#[derive(Debug, Clone)]
pub struct TenantId(pub ObjectId);

/// The ObjectId of the secret key used for authentication.
/// Handlers extract this via `Extension<AuthKeyId>`.
#[derive(Debug, Clone)]
pub struct AuthKeyId(pub ObjectId);

/// Domain associated with an SDK key, injected by `sdk_auth_gate`.
#[derive(Debug, Clone)]
pub struct SdkDomain(pub String);

/// Scope the calling key carries.
///
/// Always injected as `Extension<CallerScope>`, with `scope: None` for
/// pre-migration rows that haven't been backfilled yet (grandfathered to
/// `Full` per `services/auth/scope::require_full`). Affiliate-scoped keys
/// can only hit the path allowlist; everything else returns 403.
#[derive(Debug, Clone)]
pub struct CallerScope(pub Option<KeyScope>);

/// Auth + rate-limit middleware for protected endpoints.
///
/// Priority:
/// 1. API key present -> validate, inject TenantId, proceed
/// 2. x402 payment header -> verify with facilitator, proceed, settle after
/// 3. No key, within IP daily limit -> proceed (anonymous free tier)
/// 4. No key, IP limit exceeded -> 429
pub async fn auth_gate(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let usage_repo = match &state.usage_repo {
        Some(r) => r.as_ref(),
        None => return next.run(req).await,
    };

    let ip = client_ip(&req);
    let endpoint = req.uri().path().to_string();

    // ── Path 1: API key ──
    if let Some(raw_key) = extract_bearer(&req) {
        let (tenant_id, key_id, scope) =
            match validate_api_key(state.secret_keys_repo.as_deref(), &raw_key).await {
                Ok(ids) => ids,
                Err(resp) => return resp,
            };

        // Affiliate-scoped keys can only hit the link-minting allowlist.
        // Defense in depth: services that own affiliate-side logic also
        // call `services::auth::scope::require_*` so MCP and other
        // transports stay honest. The middleware is the coarse, fast
        // fail-closed for HTTP.
        if let Some(KeyScope::Affiliate { .. }) = scope {
            if !is_path_allowed_for_affiliate(req.method(), req.uri().path()) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "This key's scope does not permit this operation",
                        "code": "forbidden_scope"
                    })),
                )
                    .into_response();
            }
        }

        // Inject tenant identity, key identity, and scope for downstream handlers.
        req.extensions_mut().insert(TenantId(tenant_id));
        req.extensions_mut().insert(AuthKeyId(key_id));
        req.extensions_mut().insert(CallerScope(scope));

        let response = next.run(req).await;
        if response.status().is_success() {
            usage_repo
                .record_usage(usage_repo::UsageDoc {
                    id: None,
                    api_key_id: Some(key_id),
                    ip: ip.clone(),
                    endpoint: endpoint.clone(),
                    ts: usage_repo::now_bson(),
                })
                .await;
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
            usage_repo
                .record_usage(usage_repo::UsageDoc {
                    id: None,
                    api_key_id: None,
                    ip: ip.clone(),
                    endpoint: endpoint.clone(),
                    ts: usage_repo::now_bson(),
                })
                .await;
        }
        return response;
    }

    // ── Path 3: Anonymous / IP rate limit ──
    if let Err(resp) = check_anonymous_limit(&state, usage_repo, &ip).await {
        return resp;
    }

    let response = next.run(req).await;
    if response.status().is_success() {
        usage_repo
            .record_usage(usage_repo::UsageDoc {
                id: None,
                api_key_id: None,
                ip: ip.clone(),
                endpoint: endpoint.clone(),
                ts: usage_repo::now_bson(),
            })
            .await;
    }
    response
}

/// SDK auth middleware for pk_live_ keys.
///
/// Extracts the SDK bearer token, validates it against the SDK keys repository,
/// and injects TenantId and SdkDomain extensions.
pub async fn sdk_auth_gate(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(sdk_keys_repo) = &state.sdk_keys_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "SDK keys not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Accept pk_live_ key from Authorization header or ?key= query param.
    // Query param fallback enables sendBeacon (which can't set headers).
    let raw_key = extract_sdk_bearer(&req).or_else(|| extract_sdk_query_key(&req));
    let Some(raw_key) = raw_key else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing or invalid SDK key", "code": "unauthorized" })),
        )
            .into_response();
    };

    let hash = keys::hash_key(&raw_key);
    let doc = match sdk_keys_repo.find_by_hash(&hash).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid SDK key", "code": "invalid_key" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("SDK key lookup failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    if doc.revoked {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "SDK key has been revoked", "code": "key_revoked" })),
        )
            .into_response();
    }

    req.extensions_mut().insert(TenantId(doc.tenant_id));
    req.extensions_mut().insert(SdkDomain(doc.domain));

    next.run(req).await
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

fn extract_sdk_bearer(req: &Request) -> Option<String> {
    let header = req.headers().get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    token.starts_with("pk_live_").then(|| token.to_string())
}

fn extract_sdk_query_key(req: &Request) -> Option<String> {
    req.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|pair| pair.strip_prefix("key="))
            .filter(|v| v.starts_with("pk_live_"))
            .map(|v| v.to_string())
    })
}

/// Validate an API key against SecretKeysRepository.
/// Returns (tenant_id, key_id, scope).
async fn validate_api_key(
    secret_keys_repo: Option<&dyn SecretKeysRepository>,
    raw_key: &str,
) -> Result<(ObjectId, ObjectId, Option<KeyScope>), Response> {
    let hash = keys::hash_key(raw_key);

    let sk_repo = secret_keys_repo.ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Auth not configured", "code": "no_database" })),
        )
            .into_response()
    })?;

    let key_doc = sk_repo
        .find_by_hash(&hash)
        .await
        .map_err(|e| {
            tracing::error!("Key lookup failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid or unverified API key",
                    "code": "invalid_key"
                })),
            )
                .into_response()
        })?;

    if key_doc.scope.is_none() {
        // Migration-window grandfather. After m004 runs in prod and the
        // follow-up PR ships, this branch becomes a 401.
        tracing::warn!(
            key_id = %key_doc.id,
            "secret_key_missing_scope_grandfathered_to_full"
        );
    }

    Ok((key_doc.tenant_id, key_doc.id, key_doc.scope))
}

/// Path allowlist for `KeyScope::Affiliate`.
///
/// In v1 affiliate-scoped credentials can:
///   - mint a link (server pins `affiliate_id` to the scope's affiliate)
///   - read one of their own links by id
///
/// Everything else (managing affiliates, webhooks, domains, team, conversion
/// sources, etc.) requires `KeyScope::Full`. New affiliate-side endpoints
/// must be added here AND have a service-layer check.
fn is_path_allowed_for_affiliate(method: &axum::http::Method, path: &str) -> bool {
    use axum::http::Method;

    match (method, path) {
        // POST /v1/links — server pins affiliate_id at the service layer.
        (m, "/v1/links") if m == Method::POST => true,
        // GET /v1/links/{link_id} — service layer 404s on cross-affiliate.
        (m, p) if m == Method::GET && is_link_get_path(p) => true,
        _ => false,
    }
}

fn is_link_get_path(path: &str) -> bool {
    // Match `/v1/links/<segment>` exactly — no trailing slashes, no nested paths.
    let Some(rest) = path.strip_prefix("/v1/links/") else {
        return false;
    };
    !rest.is_empty() && !rest.contains('/')
}

async fn check_anonymous_limit(
    state: &AppState,
    usage_repo: &dyn usage_repo::UsageRepository,
    ip: &str,
) -> Result<(), Response> {
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let ip_used = usage_repo.count_ip_usage_since(ip, today_start).await;
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

#[allow(clippy::result_large_err)]
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
