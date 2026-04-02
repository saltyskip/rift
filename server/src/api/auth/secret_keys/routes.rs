use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::auth::middleware::{AuthKeyId, TenantId};
use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::repo::{self, ApiKeyDoc};
use crate::services::auth::secret_keys::service::SecretKeyError;
use crate::services::auth::users::service::UserError;

// ── Signup / Verify types ──

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignupResponse {
    pub message: String,
    pub key_prefix: String,
    pub note: String,
}

#[derive(Deserialize, ToSchema)]
pub struct VerifyQuery {
    pub token: String,
}

// ── Secret Key CRUD types ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestCreateKeyRequest {
    /// Email of a verified user on this tenant who will receive the confirmation code.
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmCreateKeyRequest {
    /// Email of the user who received the confirmation code.
    #[schema(example = "alice@example.com")]
    pub email: String,
    /// The 6-character confirmation code from the email.
    #[schema(example = "ABC123")]
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateKeyResponse {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    /// The full secret key. Shown only once at creation time.
    #[schema(example = "rl_live_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2")]
    pub key: String,
    #[schema(example = "rl_live_a1b2c3d4...")]
    pub key_prefix: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretKeyDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "rl_live_a1b2c3d4...")]
    pub key_prefix: String,
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0f")]
    pub created_by: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSecretKeysResponse {
    pub keys: Vec<SecretKeyDetail>,
}

// ── Signup / Verify handlers ──

#[utoipa::path(
    post,
    path = "/v1/auth/signup",
    tag = "Signup",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "Verification email sent", body = SignupResponse),
        (status = 400, description = "Invalid email", body = crate::error::ErrorResponse),
        (status = 409, description = "Email already registered", body = crate::error::ErrorResponse),
    )
)]
#[tracing::instrument(skip(state, body))]
pub async fn signup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SignupRequest>,
) -> Response {
    // Use new service if available, fall back to old
    if let Some(users_svc) = &state.users_service {
        return match users_svc
            .signup(
                &body.email,
                &state.config.public_url,
                &state.config.resend_api_key,
                &state.config.resend_from_email,
            )
            .await
        {
            Ok(_) => (
                StatusCode::CREATED,
                Json(json!({
                    "message": "Verification email sent. Click the link to verify your email and receive your API key.",
                    "note": "Your API key will be shown once after verification. Save it — we can't show it again."
                })),
            )
                .into_response(),
            Err(e) => user_error_response(&e),
        };
    }

    // ── Legacy path (old AuthRepository) ──
    let auth_repo = match &state.auth_repo {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Auth not configured", "code": "no_database" })),
            )
                .into_response();
        }
    };

    let email = body.email.trim().to_lowercase();

    if !email.contains('@') || email.len() < 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid email address", "code": "invalid_email" })),
        )
            .into_response();
    }

    if let Some(existing) = auth_repo.find_key_by_email(&email).await {
        if existing.verified {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Email already registered. Check your inbox for the original key, or contact support.",
                    "code": "email_exists"
                })),
            )
                .into_response();
        }
    }

    let (full_key, key_hash, key_prefix) = keys::generate_api_key();
    let verify_token = keys::generate_verify_token();

    let doc = ApiKeyDoc {
        id: None,
        email: email.clone(),
        key_hash,
        key_prefix: key_prefix.clone(),
        verified: false,
        verify_token: Some(verify_token.clone()),
        monthly_quota: 100,
        created_at: repo::now_bson(),
    };

    if let Err(e) = auth_repo.upsert_key(&doc).await {
        tracing::error!("Failed to upsert API key: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    let verify_url = format!(
        "{}/v1/auth/verify?token={verify_token}",
        state.config.public_url
    );

    if let Err(e) = crate::core::email::send_email(
        &state.config.resend_api_key,
        &state.config.resend_from_email,
        &email,
        "Verify your Rift API key",
        &format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">Verify your email</h2>
                <p>Click the button below to activate your Rift API key:</p>
                <a href="{verify_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Verify Email</a>
                <p style="margin-top: 24px;">Your API key:</p>
                <code style="display: block; padding: 12px; background: #f4f4f5; border-radius: 6px; word-break: break-all; font-size: 14px;">{full_key}</code>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">Save this key — we can't show it again. The key won't work until you verify your email.</p>
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
        ),
    )
    .await
    {
        tracing::error!("Failed to send verification email: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to send verification email", "code": "email_error" })),
        )
            .into_response();
    }

    tracing::info!(email = %email, "Signup: verification email sent");

    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Verification email sent. Click the link to activate your API key.",
            "key_prefix": key_prefix,
            "note": "Your full API key is included in the verification email. Save it — we can't show it again."
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/auth/verify",
    tag = "Signup",
    params(
        ("token" = String, Query, description = "Verification token from the email link"),
    ),
    responses(
        (status = 200, description = "Email verified, key activated"),
        (status = 400, description = "Invalid or expired token", body = crate::error::ErrorResponse),
    )
)]
#[tracing::instrument(skip(state, params))]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Query(params): Query<VerifyQuery>,
) -> Response {
    // Use new service if available
    if let Some(users_svc) = &state.users_service {
        return match users_svc.verify(&params.token).await {
            Ok(result) => {
                if let (Some(key), Some(prefix)) = (result.key, result.key_prefix) {
                    tracing::info!(tenant_id = %result.tenant_id, "Owner verified, key created");
                    (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Email verified! Your API key is below. Save it — we can't show it again.",
                            "code": "verified",
                            "key": key,
                            "key_prefix": prefix,
                        })),
                    )
                        .into_response()
                } else {
                    tracing::info!(tenant_id = %result.tenant_id, email = %result.email, "Invited user verified");
                    (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Email verified! You now have access to this team.",
                            "code": "verified"
                        })),
                    )
                        .into_response()
                }
            }
            Err(UserError::NotFound) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid or expired verification token",
                    "code": "invalid_token"
                })),
            )
                .into_response(),
            Err(e) => user_error_response(&e),
        };
    }

    // ── Legacy path ──
    let auth_repo = match &state.auth_repo {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Auth not configured", "code": "no_database" })),
            )
                .into_response();
        }
    };

    if auth_repo.verify_key(&params.token).await {
        (
            StatusCode::OK,
            Json(json!({
                "message": "Email verified! Your API key is now active.",
                "code": "verified"
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid or expired verification token",
                "code": "invalid_token"
            })),
        )
            .into_response()
    }
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
#[tracing::instrument(skip(state, req))]
pub async fn request_create_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<RequestCreateKeyRequest>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc
        .request_create(
            tenant.0,
            &req.email,
            &state.config.resend_api_key,
            &state.config.resend_from_email,
        )
        .await
    {
        Ok(()) => {
            let email = req.email.trim().to_lowercase();
            tracing::info!(email = %email, tenant_id = %tenant.0, "Key creation code sent");
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
#[tracing::instrument(skip(state, req))]
pub async fn confirm_create_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<ConfirmCreateKeyRequest>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.confirm_create(tenant.0, &req.email, &req.token).await {
        Ok(created) => {
            tracing::info!(tenant_id = %tenant.0, "New secret key created via email confirmation");
            (
                StatusCode::CREATED,
                Json(json!(CreateKeyResponse {
                    id: created.id.to_hex(),
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
#[tracing::instrument(skip(state))]
pub async fn list_secret_keys(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.list(&tenant.0).await {
        Ok(keys) => {
            let details: Vec<SecretKeyDetail> = keys
                .iter()
                .map(|k| SecretKeyDetail {
                    id: k.id.to_hex(),
                    key_prefix: k.key_prefix.clone(),
                    created_by: k.created_by.to_hex(),
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
#[tracing::instrument(skip(state))]
pub async fn delete_secret_key(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    axum::Extension(auth_key): axum::Extension<AuthKeyId>,
    Path(key_id): Path<String>,
) -> Response {
    let Some(svc) = &state.secret_keys_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&key_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid key ID", "code": "bad_request" })),
        )
            .into_response();
    };

    match svc.delete(tenant.0, oid, auth_key.0).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => sk_error_response(&e),
    }
}

// ── Error response helpers ──

fn user_error_response(e: &UserError) -> Response {
    let status = match e {
        UserError::InvalidEmail => StatusCode::BAD_REQUEST,
        UserError::EmailExists => StatusCode::CONFLICT,
        UserError::UserExists => StatusCode::CONFLICT,
        UserError::LastUser => StatusCode::CONFLICT,
        UserError::NotFound => StatusCode::NOT_FOUND,
        UserError::EmailFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        UserError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({ "error": e.to_string(), "code": e.code() })),
    )
        .into_response()
}

fn sk_error_response(e: &SecretKeyError) -> Response {
    let status = match e {
        SecretKeyError::UserNotMember | SecretKeyError::UserUnverified => StatusCode::FORBIDDEN,
        SecretKeyError::KeyLimit | SecretKeyError::LastKey | SecretKeyError::SelfDelete => {
            StatusCode::CONFLICT
        }
        SecretKeyError::RequestPending | SecretKeyError::TooManyAttempts => {
            StatusCode::TOO_MANY_REQUESTS
        }
        SecretKeyError::NoPendingRequest | SecretKeyError::InvalidCode => StatusCode::BAD_REQUEST,
        SecretKeyError::NotFound => StatusCode::NOT_FOUND,
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
