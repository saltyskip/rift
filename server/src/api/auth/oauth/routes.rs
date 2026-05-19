//! OAuth federation handlers — thin wrappers around `OauthService`.
//!
//! Two endpoints, both public (state token + provider exchange are the
//! security boundary, not a session cookie):
//!
//! - `GET /v1/auth/oauth/{provider}/start?next=…` — issues state token, 302
//!   to the provider's authorize URL.
//! - `GET /v1/auth/oauth/{provider}/callback?code=&state=` — consumes state,
//!   exchanges code for verified email, mints a session via
//!   `SessionsService::issue_session`, sets the cookie, 303 to the original
//!   `next` (or `/account`).
//!
//! Errors from `OauthService` are mapped to `?error=<code>` redirects so the
//! frontend can show a coherent toast instead of a raw HTTP error page. Codes
//! match `OauthError::code()` (e.g. `oauth_email_unverified`).

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use super::models::{CallbackQuery, StartQuery};
use crate::api::auth::sessions::routes::{build_cookie, redirect_to, sanitize_next};
use crate::app::AppState;
use crate::core::http::client_ip_from_headers;
use crate::core::origin::OriginMatcher;
use crate::services::auth::oauth::{OauthError, OauthProvider};
use crate::services::auth::sessions::service::SessionsService;

// ── GET /v1/auth/oauth/{provider}/start ──

#[utoipa::path(
    get,
    path = "/v1/auth/oauth/{provider}/start",
    tag = "Sessions",
    params(
        ("provider" = String, Path, description = "`github` or `google`"),
        ("next" = Option<String>, Query, description = "Optional same-origin path on the marketing site to redirect to after sign-in. Defaults to `/account`."),
    ),
    responses(
        (status = 302, description = "Redirect to the provider's authorize URL with state + PKCE challenge"),
        (status = 404, description = "Unknown provider", body = crate::error::ErrorResponse),
        (status = 503, description = "Provider not configured (missing client_id / client_secret env)", body = crate::error::ErrorResponse),
    )
)]
#[tracing::instrument(skip(state, headers))]
pub async fn start(
    State(state): State<Arc<AppState>>,
    Path(provider_str): Path<String>,
    Query(q): Query<StartQuery>,
    headers: HeaderMap,
) -> Response {
    let Some(provider) = OauthProvider::from_path_segment(&provider_str) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Unknown provider", "code": "unknown_provider" })),
        )
            .into_response();
    };

    let Some(svc) = state.oauth_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "OAuth not configured", "code": "oauth_not_configured" })),
        )
            .into_response();
    };

    // Resolve where this flow started so the callback redirects back to
    // the same browser tab. Unlike magic-link signin (which is a fetch POST
    // and always carries Origin), OAuth start is a top-level GET navigation
    // — browsers do NOT send `Origin` on those. Fall back to `Referer`,
    // which IS sent on cross-origin nav (the default `strict-origin-when-
    // cross-origin` policy sends just the origin part, which is exactly
    // what we need). Both candidates are validated against the matcher;
    // anything not allowlisted gets dropped (callback falls back to
    // `marketing_url`).
    let origin = resolve_request_origin(&headers, &state.origin_matcher);

    // Sanitize `next` against the redirect base we'll use at callback time.
    // We use `origin` if present, otherwise `marketing_url`. `sanitize_next`
    // rejects cross-origin and control-char attempts; we just stash the
    // result in the state token's metadata.
    let next_base = origin.as_deref().unwrap_or(&state.config.marketing_url);
    let next = q.next.as_deref().and_then(|n| sanitize_next(next_base, n));

    let ip = client_ip_from_headers(&headers);

    match svc
        .start(provider, &ip, next.as_deref(), origin.as_deref())
        .await
    {
        Ok(outcome) => redirect_to(&outcome.authorize_url, None),
        Err(OauthError::NotConfigured) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({ "error": "OAuth provider not configured", "code": "oauth_not_configured" }),
            ),
        )
            .into_response(),
        Err(OauthError::RateLimited) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Too many sign-in requests. Try again later.",
                "code": "rate_limited",
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, provider = %provider, "oauth_start_failed");
            let url = format!("{}/signin?error={}", state.config.marketing_url, e.code());
            redirect_to(&url, None)
        }
    }
}

// ── GET /v1/auth/oauth/{provider}/callback ──

#[utoipa::path(
    get,
    path = "/v1/auth/oauth/{provider}/callback",
    tag = "Sessions",
    params(
        ("provider" = String, Path, description = "`github` or `google`"),
        ("code" = Option<String>, Query, description = "Authorization code from the provider"),
        ("state" = Option<String>, Query, description = "Single-use state token we issued at `/start`"),
        ("error" = Option<String>, Query, description = "Set by the provider when the user cancels or denies"),
    ),
    responses(
        (status = 303, description = "Session created — redirects to `next` (or `/account`) with `Set-Cookie: session_token=…`"),
        (status = 303, description = "Anything fails — redirects to `/signin?error=<code>`"),
    )
)]
#[tracing::instrument(skip(state, headers))]
pub async fn callback(
    State(state): State<Arc<AppState>>,
    Path(provider_str): Path<String>,
    Query(q): Query<CallbackQuery>,
    headers: HeaderMap,
) -> Response {
    let fallback_base = state.config.marketing_url.as_str();
    let signin_with_error = |code: &str| {
        let url = format!("{fallback_base}/signin?error={code}");
        redirect_to(&url, None)
    };

    let Some(provider) = OauthProvider::from_path_segment(&provider_str) else {
        return signin_with_error("oauth_provider_unknown");
    };

    // Provider-initiated error (user clicked "Cancel" or "Deny"). Pass
    // through as a soft failure with a generic toast — the provider's own
    // error string is too noisy to surface verbatim.
    if let Some(err) = q.error.as_deref() {
        tracing::info!(provider = %provider, provider_error = %err, "oauth_callback_provider_error");
        return signin_with_error("oauth_provider_error");
    }

    let Some(svc) = state.oauth_service.as_ref() else {
        return signin_with_error("oauth_not_configured");
    };
    let Some(sessions) = state.sessions_service.as_ref() else {
        return signin_with_error("oauth_internal");
    };

    let (Some(code), Some(state_token)) = (q.code.as_deref(), q.state.as_deref()) else {
        return signin_with_error("oauth_state_invalid");
    };

    let outcome = match svc.consume_callback(provider, code, state_token).await {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, provider = %provider, "oauth_callback_failed");
            return signin_with_error(e.code());
        }
    };

    // Mint a session — same path as the magic-link callback ends in.
    let ip = client_ip_from_headers(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_session = match sessions
        .issue_session(
            outcome.user_id,
            outcome.tenant_id,
            Some(&ip),
            user_agent.as_deref(),
        )
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "oauth_session_issue_failed");
            return signin_with_error("oauth_internal");
        }
    };

    // Pick the redirect base. Re-validate the origin captured at /start
    // against the *current* matcher — same defense-in-depth as the
    // magic-link callback.
    let base = outcome
        .origin
        .as_deref()
        .filter(|o| state.origin_matcher.matches_str(o))
        .unwrap_or(fallback_base);
    let success_url = format!("{base}{}", outcome.next);

    let cookie = build_cookie(
        &raw_session,
        SessionsService::SESSION_TTL_SECS,
        state.config.cookie_domain.as_deref(),
        state.config.cookie_secure,
        state.config.cookie_same_site,
    );

    tracing::info!(
        user_id = %outcome.user_id,
        tenant_id = %outcome.tenant_id,
        provider = %provider,
        redirect_base = %base,
        "oauth_session_created"
    );
    redirect_to(&success_url, Some(cookie))
}

// ── Helpers ──

/// Resolve where the user is signing in from. OAuth `/start` is a top-level
/// GET navigation, so `Origin` is typically absent — fall back to `Referer`,
/// which browsers DO send on cross-origin nav (just the origin portion under
/// the default `strict-origin-when-cross-origin` policy). Both candidates
/// are passed through `OriginMatcher` so anything off the CORS allowlist is
/// rejected up front and never reaches the state token's metadata.
fn resolve_request_origin(headers: &HeaderMap, matcher: &OriginMatcher) -> Option<String> {
    if let Some(o) = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .filter(|s| matcher.matches_str(s))
    {
        return Some(o.to_string());
    }

    // Parse Referer with `url::Url` and emit the origin form ourselves —
    // browsers may include a path under non-default referrer policies, and
    // the matcher only knows how to compare origins.
    let referer = headers.get("referer").and_then(|v| v.to_str().ok())?;
    let parsed = url::Url::parse(referer).ok()?;
    let host = parsed.host_str()?;
    let origin_str = match parsed.port() {
        Some(p) => format!("{}://{}:{}", parsed.scheme(), host, p),
        None => format!("{}://{}", parsed.scheme(), host),
    };
    matcher.matches_str(&origin_str).then_some(origin_str)
}
