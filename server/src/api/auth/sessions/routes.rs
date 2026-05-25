//! Session-related HTTP handlers.
//!
//! All handlers are thin wrappers around `SessionsService` /
//! `SecretKeysService` per CLAUDE.md. Cookies are constructed inline (small
//! function, single call site each) rather than abstracted into a dedicated
//! cookie service.

use axum::extract::{Form, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use super::models::{
    CallbackForm, CallbackQuery, IssueKeyRequest, MeResponse, SignInRequest, SignInResponse,
    TenantSummary, UserSummary,
};
use crate::api::auth::models::{SessionId, UserId};
use crate::api::auth::secret_keys::models::CreateKeyResponse;
use crate::app::AppState;
use crate::core::config::CookieSameSite;
use crate::core::http::client_ip_from_headers;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::sessions::SessionError;

const SESSION_COOKIE_NAME: &str = "session_token";
/// Cookie `Max-Age` always tracks the server-side session row's TTL.
const SESSION_COOKIE_MAX_AGE: i64 =
    crate::services::auth::sessions::service::SessionsService::SESSION_TTL_SECS;

// ── POST /v1/auth/signin ──

#[utoipa::path(
    post,
    path = "/v1/auth/signin",
    tag = "Sessions",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "Always returned on validation success (prevents email enumeration)", body = SignInResponse),
        (status = 400, description = "Invalid email", body = crate::error::ErrorResponse),
        (status = 429, description = "Per-IP rate limit exceeded", body = crate::error::ErrorResponse),
        (status = 503, description = "Sessions not configured", body = crate::error::ErrorResponse),
    )
)]
#[tracing::instrument(skip(state, headers, body))]
pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SignInRequest>,
) -> Response {
    let Some(svc) = state.sessions_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Sessions not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let ip = client_ip_from_headers(&headers);

    // Capture + validate the request's Origin so the callback redirects
    // back to wherever the signin started. Validation reuses the CORS
    // allowlist (`OriginMatcher`) so the redirect-target rule and the
    // CORS rule can never disagree — anything CORS allows is allowed as
    // a redirect; everything else is dropped (callback falls back to
    // `marketing_url`).
    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .filter(|s| state.origin_matcher.matches_str(s));

    // Sanitize `next` against the resolved base (validated origin if present,
    // else marketing_url) — same precedence the callback uses. Storing the
    // pre-sanitized value in the token's metadata means the callback can
    // trust it without re-validating the user-controlled query string.
    let next_base = origin.unwrap_or(state.config.marketing_url.as_str());
    let next = body
        .next
        .as_deref()
        .and_then(|n| sanitize_next(next_base, n));

    match svc
        .request_sign_in(&body.email, &ip, origin, next.as_deref())
        .await
    {
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
        Err(e) => {
            tracing::error!(error = %e, "signin_request_failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string(), "code": e.code() })),
            )
                .into_response()
        }
    }
}

// ── GET /v1/auth/callback?token=… ──
//
// Renders an interstitial HTML page with a POST form. Doesn't touch the
// token — that happens in `callback_confirm`. The split exists because
// corporate email security products (Avanan, Microsoft Defender Safe Links,
// ProofPoint URL Defense, Mimecast) pre-fetch links via GET to scan them,
// and would consume the single-use token before the human ever clicked.
// Scanners don't submit forms, so the POST is human-only.

#[utoipa::path(
    get,
    path = "/v1/auth/callback",
    tag = "Sessions",
    params(
        ("token" = String, Query, description = "Signin token from the magic-link email"),
        ("next" = Option<String>, Query, description = "Optional same-origin path to redirect to after sign-in"),
    ),
    responses(
        (status = 200, description = "HTML interstitial — user clicks Continue to POST and consume the token", content_type = "text/html"),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let action = format!("{}/v1/auth/callback", state.config.public_url);
    let html = interstitial_html(&action, &q.token, q.next.as_deref());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        // Belt-and-braces: scanners shouldn't be caching this and neither
        // should browsers — every render is for a one-shot token.
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Robots-Tag", "noindex")
        .body(axum::body::Body::from(html))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── POST /v1/auth/callback ──

#[utoipa::path(
    post,
    path = "/v1/auth/callback",
    tag = "Sessions",
    request_body(content = CallbackForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 303, description = "Session created — redirects to `?next` (or `/account`) with a `Set-Cookie: session_token=…` header"),
        (status = 303, description = "Token invalid or expired — redirects to `/signin?error=link_expired`"),
    )
)]
#[tracing::instrument(skip(state, headers, form))]
pub async fn callback_confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<CallbackForm>,
) -> Response {
    // The redirect base used for the "link expired" path — needs a
    // sensible default before we have an outcome to read the origin from.
    let fallback_base = state.config.marketing_url.as_str();
    let expired_url = format!("{fallback_base}/signin?error=link_expired");

    let Some(svc) = state.sessions_service.as_ref() else {
        return redirect_to(&expired_url, None);
    };

    let ip = client_ip_from_headers(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match svc
        .consume_sign_in(&form.token, Some(ip.as_str()), user_agent.as_deref())
        .await
    {
        Ok(outcome) => {
            // Pick the redirect base. Token origin wins IF it still
            // passes the matcher (defense in depth — env vars may have
            // changed between signin and callback). Otherwise fall
            // back to marketing_url.
            let base = outcome
                .origin
                .as_deref()
                .filter(|o| state.origin_matcher.matches_str(o))
                .unwrap_or(fallback_base);

            // Prefer the `next` captured at signin time (already sanitized
            // against its base, can't be tampered with by whoever clicked the
            // emailed link). Fall back to a form-body `next` (carried through
            // the interstitial from the original query string) and finally
            // `/account`.
            let success_path = outcome
                .next
                .clone()
                .or_else(|| form.next.as_deref().and_then(|n| sanitize_next(base, n)))
                .unwrap_or_else(|| "/account".to_string());
            let success_url = format!("{base}{success_path}");

            let cookie = build_cookie(
                &outcome.raw_token,
                SESSION_COOKIE_MAX_AGE,
                state.config.cookie_domain.as_deref(),
                state.config.cookie_secure,
                state.config.cookie_same_site,
            );
            tracing::info!(
                user_id = %outcome.user_id,
                tenant_id = %outcome.tenant_id,
                redirect_base = %base,
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

#[utoipa::path(
    get,
    path = "/v1/auth/me",
    tag = "Sessions",
    responses(
        (status = 200, description = "Resolved user + tenant for the active session", body = MeResponse),
        (status = 401, description = "No active session (or session expired/revoked)", body = crate::error::ErrorResponse),
    ),
    security(("session_cookie" = []))
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn me(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    axum::Extension(user): axum::Extension<UserId>,
) -> Response {
    let Some(users_repo) = state.users_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Users not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Fetch by tenant+user_id via the existing list path — there's no
    // find_by_id helper today and Phase 1 doesn't justify adding one. The
    // list is bounded by team size (currently 1-5 users per tenant).
    let users = match users_repo.list(&ctx).await {
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
            id: ctx.tenant_id.to_hex(),
        },
    })
    .into_response()
}

// ── POST /v1/auth/signout ──

#[utoipa::path(
    post,
    path = "/v1/auth/signout",
    tag = "Sessions",
    responses(
        (status = 204, description = "Session revoked; `Set-Cookie` clears `session_token`"),
        (status = 401, description = "No active session", body = crate::error::ErrorResponse),
    ),
    security(("session_cookie" = []))
)]
#[tracing::instrument(skip(state))]
pub async fn sign_out(
    State(state): State<Arc<AppState>>,
    axum::Extension(session): axum::Extension<SessionId>,
) -> Response {
    let Some(svc) = state.sessions_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Sessions not configured", "code": "no_database" })),
        )
            .into_response();
    };

    if let Err(e) = svc.revoke(&session.0).await {
        tracing::error!(error = %e, "signout_failed");
    }

    // Clearing the cookie = same attrs as a fresh issue but with an empty
    // value and Max-Age=0.
    let cookie = build_cookie(
        "",
        0,
        state.config.cookie_domain.as_deref(),
        state.config.cookie_secure,
        state.config.cookie_same_site,
    );
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(header::SET_COOKIE, cookie)
        .body(axum::body::Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── POST /v1/auth/secret-keys/issue ──

#[utoipa::path(
    post,
    path = "/v1/auth/secret-keys/issue",
    tag = "Secret Keys",
    request_body = IssueKeyRequest,
    responses(
        (status = 201, description = "Key minted — full secret returned once", body = crate::api::auth::secret_keys::models::CreateKeyResponse),
        (status = 401, description = "No active session", body = crate::error::ErrorResponse),
        (status = 409, description = "Per-tenant key limit reached", body = crate::error::ErrorResponse),
        (status = 503, description = "Secret keys not configured", body = crate::error::ErrorResponse),
    ),
    security(("session_cookie" = []))
)]
#[tracing::instrument(skip(state, ctx, _body))]
pub async fn issue_secret_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(_body): Json<IssueKeyRequest>,
) -> Response {
    let Some(svc) = state.secret_keys_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Secret keys not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.issue_for_session(&ctx).await {
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

pub(crate) fn redirect_to(url: &str, cookie: Option<String>) -> Response {
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

/// Construct a `Set-Cookie` value for the session cookie.
///
/// Same function for issuing fresh cookies (pass the raw token + a positive
/// `max_age`) and for clearing on signout (pass empty value + `max_age=0`);
/// attrs match so browsers actually scrub the cookie they previously set.
///
/// `domain`, `secure`, and `same_site` are pre-resolved by `Config::from_env`
/// — see `resolve_cookie_domain` and `CookieSameSite::from_env_str`. Prod
/// gets `Domain=.riftl.ink; Secure; SameSite=Lax`. Sandbox gets either the
/// same shape (when API + marketing share `.sandbox.riftl.ink`) or
/// `SameSite=None; Secure` with no Domain (when marketing lives on a
/// different parent, e.g. a Vercel preview URL).
pub(crate) fn build_cookie(
    value: &str,
    max_age: i64,
    domain: Option<&str>,
    secure: bool,
    same_site: CookieSameSite,
) -> String {
    // Conventional attr order: name=value; Domain=...; Path=/; HttpOnly;
    // Secure; SameSite=...; Max-Age=N
    let mut out = format!("{SESSION_COOKIE_NAME}={value}");
    if let Some(d) = domain {
        out.push_str("; Domain=");
        out.push_str(d);
    }
    out.push_str("; Path=/; HttpOnly");
    if secure {
        out.push_str("; Secure");
    }
    out.push_str("; SameSite=");
    out.push_str(same_site.as_str());
    out.push_str("; Max-Age=");
    out.push_str(&max_age.to_string());
    out
}

/// Validate that `next` is a same-origin path on `base_url` and return the
/// safe `path[?query]` to redirect to.
///
/// The hard work is delegated to `url::Url::join` + an origin-equality check —
/// a WHATWG-conformant parser is the only correct way to detect bypasses like
/// `//evil.com` (protocol-relative), `https://evil.com` (absolute), or
/// scheme-relative variants. We additionally reject control characters
/// (whitespace, tabs, CR/LF, backslash) up front: browsers strip those
/// before parsing the `Location` header, so a URL the parser sees as
/// same-origin may not match what the browser actually navigates to.
///
/// Returns `None` (use the default `/account` fallback) for anything
/// suspicious. The caller controls the authority by concatenating its own
/// `marketing_url`; we only emit the path + query from `Url::join`.
pub(crate) fn sanitize_next(base_url: &str, next: &str) -> Option<String> {
    if next.is_empty()
        || next
            .bytes()
            .any(|b| matches!(b, b'\\' | b'\t' | b'\n' | b'\r' | 0..=0x1F))
    {
        return None;
    }

    let base = url::Url::parse(base_url).ok()?;
    let resolved = base.join(next).ok()?;

    if resolved.origin() != base.origin() {
        return None;
    }

    let mut out = resolved.path().to_string();
    if let Some(q) = resolved.query() {
        out.push('?');
        out.push_str(q);
    }
    Some(out)
}

/// Minimal HTML interstitial served by `GET /v1/auth/callback`. The form
/// POSTs back to the same path; `token` (and optional `next`) ride in hidden
/// fields. No JS, no auto-submit — a JS-rendering scanner that wants to
/// defeat this would have to click buttons, which none of the major email
/// security products do today.
fn interstitial_html(action: &str, token: &str, next: Option<&str>) -> String {
    let token_attr = html_escape(token);
    let next_field = next
        .map(|n| {
            format!(
                r#"<input type="hidden" name="next" value="{}">"#,
                html_escape(n)
            )
        })
        .unwrap_or_default();
    let action_attr = html_escape(action);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>Sign in to Rift</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; background: #fafafa; color: #18181b; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 20px; }}
  .card {{ max-width: 420px; width: 100%; background: #fff; border: 1px solid #e4e4e7; border-radius: 12px; padding: 32px; text-align: center; box-sizing: border-box; }}
  h1 {{ margin: 0 0 8px; font-size: 20px; font-weight: 600; }}
  p {{ color: #71717a; margin: 0 0 24px; font-size: 14px; line-height: 1.5; }}
  button {{ background: #0d9488; color: #fff; border: none; border-radius: 6px; padding: 12px 24px; font-size: 15px; font-weight: 500; cursor: pointer; width: 100%; }}
  button:hover {{ background: #0f766e; }}
</style>
</head>
<body>
<div class="card">
<h1>Confirm sign in</h1>
<p>Click below to finish signing in to Rift.</p>
<form method="post" action="{action_attr}">
<input type="hidden" name="token" value="{token_attr}">
{next_field}
<button type="submit">Continue</button>
</form>
</div>
</body>
</html>"#
    )
}

/// Escape the five characters that matter inside HTML attribute values.
/// The token is already URL-safe (hex), but `next` is user-controlled and
/// could carry `&` / `<` / quotes — escaping uniformly is cheaper than
/// reasoning about which inputs need it.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
