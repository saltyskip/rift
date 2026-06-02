use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use axum_extra::headers::{Cookie, HeaderMapExt};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use x402_axum::paygate::PaygateProtocol;
use x402_types::proto::v1;

use super::models::{AuthKeyId, SdkDomain};
use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::auth::usage::repo::{self as usage_repo};

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
    match try_secret_key_auth(&state, &mut req).await {
        Some(Ok(key_id)) => {
            let response = next.run(req).await;
            if response.status().is_success() {
                record_billable_usage(usage_repo, Some(key_id), &ip, &endpoint).await;
            }
            return response;
        }
        Some(Err(resp)) => return resp,
        None => {}
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
            record_billable_usage(usage_repo, None, &ip, &endpoint).await;
        }
        return response;
    }

    // ── Path 3: Anonymous / IP rate limit ──
    if let Err(resp) = check_anonymous_limit(&state, usage_repo, &ip).await {
        return resp;
    }

    let response = next.run(req).await;
    if response.status().is_success() {
        record_billable_usage(usage_repo, None, &ip, &endpoint).await;
    }
    response
}

/// Session auth middleware. Resolves the cookie `session_token` (or a non-`rl_`
/// `Authorization: Bearer` token for forward-compat) to an active session,
/// injects `TenantId` + `UserId` + `SessionId` + `CallerScope(Full)`, and
/// passes through. Sessions are always full-tenant scope — there's no
/// affiliate-scoped human in Phase 1.
pub async fn session_auth_gate(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    // Distinguish "sessions not configured" (503) from "no token" (401);
    // `try_session_auth` collapses both into `Absent`, so guard service first.
    if state.sessions_service.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Sessions not configured", "code": "no_database" })),
        )
            .into_response();
    }

    match try_session_auth(&state, &mut req).await {
        SessionOutcome::Authenticated => next.run(req).await,
        SessionOutcome::Error(resp) => resp,
        SessionOutcome::Absent => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing session", "code": "unauthorized" })),
        )
            .into_response(),
        SessionOutcome::Stale => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Session expired or revoked", "code": "unauthorized" })),
        )
            .into_response(),
    }
}

/// Auth middleware that accepts EITHER a session (cookie or non-`rl_` bearer)
/// OR an `rl_live_` API key. Used by endpoints that are equally meaningful
/// from the dashboard (session) and from automation (API key) — currently the
/// `GET`/`DELETE /v1/auth/secret-keys/*` routes.
///
/// Session wins if both are present. API-key path preserves existing affiliate
/// scope enforcement.
pub async fn session_or_key_auth_gate(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    // Captured up front because `next.run(req).await` moves the request.
    // Needed for the API-key path's usage-recording, which mirrors
    // `auth_gate`'s billable-call accounting (without this, key-authed calls to
    // /v1/auth/secret-keys/* would bypass the quota meter).
    let ip = client_ip(&req);
    let endpoint = req.uri().path().to_string();

    // ── Path A: session (wins if present; a stale cookie falls through to a key) ──
    match try_session_auth(&state, &mut req).await {
        SessionOutcome::Authenticated => return next.run(req).await,
        SessionOutcome::Error(resp) => return resp,
        SessionOutcome::Absent | SessionOutcome::Stale => {}
    }

    // ── Path B: secret key (mirrors auth_gate's key path, incl. usage recording) ──
    match try_secret_key_auth(&state, &mut req).await {
        Some(Ok(key_id)) => {
            let response = next.run(req).await;
            if response.status().is_success() {
                if let Some(usage) = state.usage_repo.as_deref() {
                    record_billable_usage(usage, Some(key_id), &ip, &endpoint).await;
                }
            }
            return response;
        }
        Some(Err(resp)) => return resp,
        None => {}
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Session or API key required", "code": "unauthorized" })),
    )
        .into_response()
}

/// Extract a session token from the `Cookie: session_token=…` header or from a
/// non-`rl_` / non-`pk_` `Authorization: Bearer …` header. The latter is
/// forward-compat for the CLI device-flow (Phase 2); browsers always use the
/// cookie.
fn extract_session_token(req: &Request) -> Option<String> {
    if let Some(cookie) = req.headers().typed_get::<Cookie>() {
        if let Some(value) = cookie.get("session_token") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    let header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())?;
    let token = header.strip_prefix("Bearer ")?;
    (!token.starts_with("rl_") && !token.starts_with("pk_") && !token.is_empty())
        .then(|| token.to_string())
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
    match try_publishable_key_auth(&state, &mut req).await {
        Some(Ok(())) => next.run(req).await,
        Some(Err(resp)) => resp,
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing or invalid SDK key", "code": "unauthorized" })),
        )
            .into_response(),
    }
}

/// Auth gate for tenant-scoped **read** endpoints reachable by both the
/// dashboard/CLI (secret key `rl_live_`) and the mobile SDK (publishable key
/// `pk_live_`). Validates whichever credential is present and injects
/// `TenantId` so the handler is auth-source-agnostic.
///
/// Unlike [`auth_gate`] there is no anonymous/x402 path — a tenant-scoped read
/// requires a real key (anonymous access could not resolve a tenant anyway).
/// Affiliate-scoped secret keys remain gated by the same path allowlist.
pub async fn auth_gate_dual_read(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    // Secret key (dashboard / CLI).
    match try_secret_key_auth(&state, &mut req).await {
        Some(Ok(_key_id)) => return next.run(req).await,
        Some(Err(resp)) => return resp,
        None => {}
    }

    // Publishable key (mobile SDK).
    match try_publishable_key_auth(&state, &mut req).await {
        Some(Ok(())) => return next.run(req).await,
        Some(Err(resp)) => return resp,
        None => {}
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Missing or invalid API key", "code": "unauthorized" })),
    )
        .into_response()
}

/// Validate a `pk_live_` key and resolve its tenant + associated domain.
/// Shared by [`sdk_auth_gate`] and [`auth_gate_dual_read`].
async fn resolve_sdk_key(
    state: &Arc<AppState>,
    raw_key: &str,
) -> Result<(crate::core::public_id::TenantId, SdkDomain), Response> {
    let Some(sdk_keys_repo) = &state.sdk_keys_repo else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "SDK keys not configured", "code": "no_database" })),
        )
            .into_response());
    };

    let hash = keys::hash_key(raw_key);
    let doc = match sdk_keys_repo.find_by_hash(&hash).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid SDK key", "code": "invalid_key" })),
            )
                .into_response());
        }
        Err(e) => {
            tracing::error!("SDK key lookup failed: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response());
        }
    };

    if doc.revoked {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "SDK key has been revoked", "code": "key_revoked" })),
        )
            .into_response());
    }

    Ok((doc.tenant_id, SdkDomain(doc.domain)))
}

// ── Credential resolvers ──
//
// Each gate above is a composition of these: "try credential A, else B, else
// reject." A resolver injects its identity extensions on success and reports
// whether the credential was absent (try the next one) or present-but-invalid
// (reject with its response). Validating each credential in exactly one place
// is what keeps the gates from drifting apart.

/// Outcome of attempting session authentication. Lets each gate apply its own
/// policy for a missing/stale session: the strict session gate 401s, while a
/// dual gate falls through to a key.
enum SessionOutcome {
    /// No session configured or no token present — try another credential.
    Absent,
    /// Session resolved; identity extensions injected.
    Authenticated,
    /// Token present but expired or revoked.
    Stale,
    /// Hard failure (e.g. DB error) — return this response as-is.
    Error(Response),
}

/// Try session auth from the cookie / non-`rl_` bearer. On success injects
/// `TenantId` + `UserId` + `SessionId` + `AuthContext::for_session` and tags
/// Sentry. Returns [`SessionOutcome::Absent`] when sessions aren't configured
/// or no token is present, so the caller decides whether that's a 401 or a
/// fall-through to a key.
async fn try_session_auth(state: &Arc<AppState>, req: &mut Request) -> SessionOutcome {
    let Some(svc) = state.sessions_service.clone() else {
        return SessionOutcome::Absent;
    };
    let Some(raw_token) = extract_session_token(req) else {
        return SessionOutcome::Absent;
    };

    match svc.lookup(&raw_token).await {
        Ok(Some(resolved)) => {
            req.extensions_mut().insert(resolved.tenant_id);
            req.extensions_mut().insert(resolved.user_id);
            req.extensions_mut().insert(resolved.session_id);
            req.extensions_mut().insert(AuthContext::for_session(
                resolved.tenant_id,
                resolved.user_id,
                resolved.session_id,
            ));
            sentry::configure_scope(|s| {
                s.set_tag("tenant_id", resolved.tenant_id.to_string());
                s.set_tag("user_id", resolved.user_id.to_string());
                s.set_tag("transport", "session");
            });
            SessionOutcome::Authenticated
        }
        Ok(None) => SessionOutcome::Stale,
        Err(e) => {
            tracing::error!(error = %e, "session_lookup_failed");
            SessionOutcome::Error(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Internal error", "code": "db_error" })),
                )
                    .into_response(),
            )
        }
    }
}

/// Try secret-key (`rl_live_`) auth. `None` if no secret-key bearer is present;
/// `Some(Ok(key_id))` after validation (injects `TenantId` + `AuthKeyId` +
/// `AuthContext::for_secret_key` and tags Sentry); `Some(Err)` if the key is
/// invalid or its affiliate scope doesn't permit this path. The returned
/// `key_id` lets the caller attribute billable usage.
async fn try_secret_key_auth(
    state: &Arc<AppState>,
    req: &mut Request,
) -> Option<Result<crate::core::public_id::SecretKeyId, Response>> {
    let raw_key = extract_bearer(req)?;

    let (tenant_id, key_id, scope) =
        match validate_api_key(state.secret_keys_repo.as_deref(), &raw_key).await {
            Ok(ids) => ids,
            Err(resp) => return Some(Err(resp)),
        };

    // Affiliate-scoped keys can only hit the link-minting allowlist. Defense in
    // depth: services that own affiliate-side logic also call
    // `services::auth::scope::require_*`; this is the coarse, fast fail-closed
    // for HTTP.
    if let Some(KeyScope::Affiliate { .. }) = scope {
        if !is_path_allowed_for_affiliate(req.method(), req.uri().path()) {
            return Some(Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "This key's scope does not permit this operation",
                    "code": "forbidden_scope"
                })),
            )
                .into_response()));
        }
    }

    req.extensions_mut().insert(tenant_id);
    req.extensions_mut().insert(AuthKeyId(key_id));
    req.extensions_mut().insert(AuthContext::for_secret_key(
        tenant_id,
        key_id,
        scope.as_ref(),
    ));

    // Per-request Sentry hub (NewSentryLayer) — request-local, no leakage.
    sentry::configure_scope(|s| {
        s.set_tag("tenant_id", tenant_id.to_string());
        s.set_tag("key_id", key_id.to_string());
        s.set_tag("transport", "http");
        if let Some(KeyScope::Affiliate { .. }) = &scope {
            s.set_tag("key_scope", "affiliate");
        }
    });

    Some(Ok(key_id))
}

/// Try publishable-key (`pk_live_`) auth from the bearer header or `?key=`
/// query param (the latter enables sendBeacon, which can't set headers).
/// `None` if absent; injects `TenantId` + `SdkDomain` on success.
async fn try_publishable_key_auth(
    state: &Arc<AppState>,
    req: &mut Request,
) -> Option<Result<(), Response>> {
    let raw_key = extract_sdk_bearer(req).or_else(|| extract_sdk_query_key(req))?;
    match resolve_sdk_key(state, &raw_key).await {
        Ok((tenant_id, sdk_domain)) => {
            req.extensions_mut().insert(tenant_id);
            req.extensions_mut().insert(sdk_domain);
            Some(Ok(()))
        }
        Err(resp) => Some(Err(resp)),
    }
}

/// Record a billable API call against the usage meter. `api_key_id` is `Some`
/// for key-authed calls, `None` for x402 / anonymous-tier calls. Session-authed
/// calls are human dashboard activity and intentionally do not call this.
async fn record_billable_usage(
    usage: &dyn usage_repo::UsageRepository,
    api_key_id: Option<crate::core::public_id::SecretKeyId>,
    ip: &str,
    endpoint: &str,
) {
    usage
        .record_usage(usage_repo::UsageDoc {
            id: None,
            api_key_id,
            ip: ip.to_string(),
            endpoint: endpoint.to_string(),
            ts: usage_repo::now_bson(),
        })
        .await;
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
) -> Result<
    (
        crate::core::public_id::TenantId,
        crate::core::public_id::SecretKeyId,
        Option<KeyScope>,
    ),
    Response,
> {
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
