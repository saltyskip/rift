use axum::extract::{Form, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use super::models::{
    ConfirmCreateKeyRequest, CreateKeyResponse, ListSecretKeysResponse, RequestCreateKeyRequest,
    SecretKeyDetail, VerifyForm, VerifyQuery,
};
use crate::api::auth::email_interstitial::{self, InterstitialContent};
use crate::app::AppState;
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::secret_keys::models::SecretKeyError;
use crate::services::auth::users::models::UserError;

// ── Team-invite verification ──
//
// GET renders an HTML interstitial; only POST consumes the token. Without
// this split, corporate email scanners (Avanan, Defender Safe Links,
// ProofPoint, Mimecast) pre-fetch the link and burn the single-use token
// before the invited user gets a chance to click. Same pattern as the
// magic-link signin callback.

#[utoipa::path(
    get,
    path = "/v1/auth/verify",
    tag = "Team",
    params(
        ("token" = String, Query, description = "Verification token from a team-invite email"),
    ),
    responses(
        (status = 200, description = "HTML interstitial — invitee clicks Accept to POST and consume the token", content_type = "text/html"),
    )
)]
#[tracing::instrument(skip(state, params))]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Query(params): Query<VerifyQuery>,
) -> Response {
    let action = format!("{}/v1/auth/verify", state.config.public_url);
    email_interstitial::render(
        &action,
        &params.token,
        None,
        InterstitialContent {
            title: "Accept invitation",
            body: "Click below to accept your invitation and join the team on Rift.",
            button: "Accept",
        },
    )
}

#[utoipa::path(
    post,
    path = "/v1/auth/verify",
    tag = "Team",
    request_body(content = VerifyForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 303, description = "Invite accepted — redirects to `${marketing_url}/signin?invite=accepted`"),
        (status = 303, description = "Token invalid or expired — redirects to `${marketing_url}/signin?error=invite_invalid`"),
    )
)]
#[tracing::instrument(skip(state, form))]
pub async fn verify_email_confirm(
    State(state): State<Arc<AppState>>,
    Form(form): Form<VerifyForm>,
) -> Response {
    let signin_url = format!("{}/signin", state.config.marketing_url);
    let success_url = format!("{signin_url}?invite=accepted");
    let invalid_url = format!("{signin_url}?error=invite_invalid");

    let Some(users_svc) = &state.users_service else {
        return redirect_to(&invalid_url);
    };

    match users_svc.verify(&form.token).await {
        Ok(result) => {
            tracing::info!(tenant_id = %result.tenant_id, email = %result.email, "Invited user verified");
            redirect_to(&success_url)
        }
        Err(UserError::NotFound) => redirect_to(&invalid_url),
        Err(e) => {
            tracing::error!(error = %e, "invite_verify_failed");
            redirect_to(&invalid_url)
        }
    }
}

fn redirect_to(url: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, url)
        .body(axum::body::Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── Secret Key CRUD handlers ──

#[utoipa::path(
    post,
    path = "/v1/auth/secret-keys",
    tag = "Secret Keys",
    request_body = RequestCreateKeyRequest,
    responses(
        (status = 200, description = "Confirmation code sent", body = serde_json::Value),
        (status = 403, description = "User not authorized", body = crate::error::ErrorResponse),
        (status = 409, description = "Key limit reached", body = crate::error::ErrorResponse),
        (status = 429, description = "Request already pending", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx, req))]
pub async fn request_create_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(req): Json<RequestCreateKeyRequest>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let tenant_id = ctx.tenant_id;
    match svc
        .request_create(
            &ctx,
            &req.email,
            &state.config.resend_api_key,
            &state.config.resend_from_email,
        )
        .await
    {
        Ok(()) => {
            let email = req.email.trim().to_lowercase();
            tracing::info!(email = %email, tenant_id = %tenant_id, "Key creation code sent");
            (
                StatusCode::OK,
                Json(json!({
                    "message": format!("Confirmation code sent to {email}"),
                    "code": "code_sent"
                })),
            )
                .into_response()
        }
        Err(e) => sk_error_response(&e),
    }
}

#[utoipa::path(
    post,
    path = "/v1/auth/secret-keys/confirm",
    tag = "Secret Keys",
    request_body = ConfirmCreateKeyRequest,
    responses(
        (status = 201, description = "Key created", body = CreateKeyResponse),
        (status = 400, description = "Invalid or expired code", body = crate::error::ErrorResponse),
        (status = 429, description = "Too many attempts", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx, req))]
pub async fn confirm_create_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(req): Json<ConfirmCreateKeyRequest>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let tenant_id = ctx.tenant_id;
    match svc.confirm_create(&ctx, &req.email, &req.token).await {
        Ok(created) => {
            tracing::info!(tenant_id = %tenant_id, "New secret key created via email confirmation");
            (
                StatusCode::CREATED,
                Json(json!(CreateKeyResponse {
                    id: created.id.to_string(),
                    key: created.key,
                    key_prefix: created.key_prefix,
                    created_at: created
                        .created_at
                        .try_to_rfc3339_string()
                        .unwrap_or_default(),
                })),
            )
                .into_response()
        }
        Err(e) => sk_error_response(&e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/auth/secret-keys",
    tag = "Secret Keys",
    responses(
        (status = 200, description = "List of secret keys", body = ListSecretKeysResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn list_secret_keys(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.list(&ctx).await {
        Ok(keys) => {
            let details: Vec<SecretKeyDetail> = keys
                .iter()
                .map(|k| SecretKeyDetail {
                    id: k.id.to_string(),
                    key_prefix: k.key_prefix.clone(),
                    created_by: k.created_by.to_string(),
                    created_at: k.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!(ListSecretKeysResponse { keys: details })).into_response()
        }
        Err(e) => sk_error_response(&e),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/auth/secret-keys/{key_id}",
    tag = "Secret Keys",
    params(("key_id" = String, Path, description = "Secret Key ID")),
    responses(
        (status = 204, description = "Key deleted"),
        (status = 404, description = "Key not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Cannot delete last key or self", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_secret_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(key_id): Path<crate::core::public_id::SecretKeyId>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Self-delete guard derives from `ctx.principal` inside the service —
    // `Principal::SecretKey` matches request key_id means self-delete; sessions
    // can't self-delete because their principal is `User`.
    match svc.delete(&ctx, key_id.to_object_id()).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => sk_error_response(&e),
    }
}

// ── Error response helpers ──

fn sk_error_response(e: &SecretKeyError) -> Response {
    if let SecretKeyError::Forbidden(authz) = e {
        return crate::api::auth::forbidden_response::to_response(authz.clone());
    }
    let status = match e {
        SecretKeyError::UserNotMember | SecretKeyError::UserUnverified => StatusCode::FORBIDDEN,
        SecretKeyError::KeyLimit | SecretKeyError::LastKey | SecretKeyError::SelfDelete => {
            StatusCode::CONFLICT
        }
        SecretKeyError::RequestPending | SecretKeyError::TooManyAttempts => {
            StatusCode::TOO_MANY_REQUESTS
        }
        SecretKeyError::InvalidCode => StatusCode::BAD_REQUEST,
        SecretKeyError::NotFound => StatusCode::NOT_FOUND,
        SecretKeyError::Forbidden(_) => unreachable!("handled above"),
        SecretKeyError::EmailFailed(_) | SecretKeyError::Internal(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(json!({ "error": e.to_string(), "code": e.code() })),
    )
        .into_response()
}
