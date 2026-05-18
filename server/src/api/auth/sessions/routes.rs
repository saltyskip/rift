//! Session-related HTTP handlers.
//!
//! All handlers are thin wrappers around `SessionsService` /
//! `SecretKeysService` per CLAUDE.md. Cookies are constructed inline (small
//! function, single call site each) rather than abstracted into a dedicated
//! cookie service.

use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use super::models::{
    CallbackQuery, IssueKeyRequest, MeResponse, SignInRequest, SignInResponse, TenantSummary,
    UserSummary,
};
use crate::api::auth::models::{SessionId, TenantId, UserId};
use crate::api::auth::secret_keys::models::CreateKeyResponse;
use crate::app::AppState;
use crate::services::auth::sessions::SessionError;

const SESSION_COOKIE_NAME: &str = "session_token";
/// 30 days, matches `SessionsService::SESSION_TTL_SECS`.
const SESSION_COOKIE_MAX_AGE: i64 = 30 * 24 * 60 * 60;

// ── POST /v1/auth/signin ──

#[tracing::instrument(skip(state, headers, body))]
pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SignInRequest>,
) -> Response {
    let Some(svc) = state.sessions_service.as_ref() else {
        return service_unavailable("Sessions not configured");
    };

    let ip = extract_client_ip(&headers);

    match svc.request_sign_in(&body.email, &ip).await {
        Ok(()) => (StatusCode::OK, Json(SignInResponse { status: "sent" })).into_response(),
        Err(SessionError::RateLimited) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Too many sign-in requests. Try again later.",
                "code": "rate_limited"
            })),
        )
            .into_response(),
        Err(SessionError::InvalidEmail) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid email address", "code": "invalid_email" })),
        )
            .into_response(),
        Err(e) => internal_error_response(&e),
    }
}

// ── GET /v1/auth/callback?token=… ──

#[tracing::instrument(skip(state, headers))]
pub async fn callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let marketing_url = &state.config.marketing_url;
    let next_path = q.next.as_deref().unwrap_or("/account");
    let next_safe = sanitize_next(next_path).unwrap_or("/account");
    let success_url = format!("{marketing_url}{next_safe}");
    let expired_url = format!("{marketing_url}/signin?error=link_expired");

    let Some(svc) = state.sessions_service.as_ref() else {
        return redirect_to(&expired_url, None);
    };

    let ip = extract_client_ip(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match svc
        .consume_sign_in(&q.token, Some(ip.as_str()), user_agent.as_deref())
        .await
    {
        Ok(outcome) => {
            let cookie = build_session_cookie(
                &outcome.raw_token,
                &state.config.environment,
                SESSION_COOKIE_MAX_AGE,
            );
            tracing::info!(
                user_id = %outcome.user_id,
                tenant_id = %outcome.tenant_id,
                "session_created"
            );
            redirect_to(&success_url, Some(cookie))
        }
        Err(SessionError::InvalidToken) => redirect_to(&expired_url, None),
        Err(e) => {
            tracing::error!(error = %e, "signin_callback_failed");
            redirect_to(&expired_url, None)
        }
    }
}

// ── GET /v1/auth/me ──

#[tracing::instrument(skip(state))]
pub async fn me(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(user): axum::Extension<UserId>,
) -> Response {
    let Some(users_repo) = state.users_service.as_ref() else {
        return service_unavailable("Users not configured");
    };

    // Fetch by tenant+user_id via the existing list path — there's no
    // find_by_id helper today and Phase 1 doesn't justify adding one. The
    // list is bounded by team size (currently 1-5 users per tenant).
    let users = match users_repo.list(&tenant.0).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, "me_user_lookup_failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    let Some(user_detail) = users.into_iter().find(|u| u.id == user.0) else {
        // Session points at a user that no longer exists. Treat as a stale
        // session — caller should re-sign-in.
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User no longer exists", "code": "unauthorized" })),
        )
            .into_response();
    };

    Json(MeResponse {
        user: UserSummary {
            id: user_detail.id.to_hex(),
            email: user_detail.email,
            verified: user_detail.verified,
            is_owner: user_detail.is_owner,
        },
        tenant: TenantSummary {
            id: tenant.0.to_hex(),
        },
    })
    .into_response()
}

// ── POST /v1/auth/signout ──

#[tracing::instrument(skip(state))]
pub async fn sign_out(
    State(state): State<Arc<AppState>>,
    axum::Extension(session): axum::Extension<SessionId>,
) -> Response {
    let Some(svc) = state.sessions_service.as_ref() else {
        return service_unavailable("Sessions not configured");
    };

    if let Err(e) = svc.revoke(&session.0).await {
        tracing::error!(error = %e, "signout_failed");
    }

    let cookie = build_cleared_session_cookie(&state.config.environment);
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(header::SET_COOKIE, cookie)
        .body(axum::body::Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── POST /v1/auth/secret-keys/issue ──

#[tracing::instrument(skip(state, _body))]
pub async fn issue_secret_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(user): axum::Extension<UserId>,
    Json(_body): Json<IssueKeyRequest>,
) -> Response {
    let Some(svc) = state.secret_keys_service.as_ref() else {
        return service_unavailable("Secret keys not configured");
    };

    match svc.issue_for_session(tenant.0, user.0).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(CreateKeyResponse {
                id: created.id.to_hex(),
                key: created.key,
                key_prefix: created.key_prefix,
                created_at: created
                    .created_at
                    .try_to_rfc3339_string()
                    .unwrap_or_default(),
            }),
        )
            .into_response(),
        Err(e) => {
            use crate::services::auth::secret_keys::models::SecretKeyError;
            let status = match e {
                SecretKeyError::KeyLimit => StatusCode::CONFLICT,
                SecretKeyError::EmailFailed(_) | SecretKeyError::Internal(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                _ => StatusCode::BAD_REQUEST,
            };
            (
                status,
                Json(json!({ "error": e.to_string(), "code": e.code() })),
            )
                .into_response()
        }
    }
}

// ── Helpers ──

fn service_unavailable(message: &'static str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": message, "code": "service_unavailable" })),
    )
        .into_response()
}

fn internal_error_response(e: &SessionError) -> Response {
    tracing::error!(error = %e, "session_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": e.to_string(), "code": e.code() })),
    )
        .into_response()
}

fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "local".to_string())
}

fn redirect_to(url: &str, cookie: Option<String>) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, url);
    if let Some(cookie) = cookie {
        builder = builder.header(header::SET_COOKIE, cookie);
    }
    builder
        .body(axum::body::Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Construct the `Set-Cookie` value for a fresh session.
///
/// `Domain=.riftl.ink` is hardcoded for prod — the marketing site and API
/// both sit under `riftl.ink`, so this scopes the cookie to share between
/// them while excluding unrelated subdomains. Dev (`environment=development`)
/// drops `Domain` and `Secure` so localhost works without HTTPS.
fn build_session_cookie(raw_token: &str, environment: &str, max_age: i64) -> String {
    if environment == "development" {
        format!(
            "{SESSION_COOKIE_NAME}={raw_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}"
        )
    } else {
        format!(
            "{SESSION_COOKIE_NAME}={raw_token}; Domain=.riftl.ink; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={max_age}"
        )
    }
}

/// Construct the `Set-Cookie` value that clears the session cookie on signout.
/// Mirrors the prod cookie attrs so browsers actually scrub it.
fn build_cleared_session_cookie(environment: &str) -> String {
    if environment == "development" {
        format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
    } else {
        format!(
            "{SESSION_COOKIE_NAME}=; Domain=.riftl.ink; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0"
        )
    }
}

/// Refuse anything that's not a simple same-site relative path. Stops
/// open-redirect via `?next=https://evil.com`. Returns `Some(safe_path)` on
/// success.
fn sanitize_next(next: &str) -> Option<&str> {
    if next.starts_with('/') && !next.starts_with("//") && !next.contains("://") {
        Some(next)
    } else {
        None
    }
}
