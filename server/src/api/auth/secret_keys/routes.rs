use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::new_repo::SecretKeyDoc;
use crate::services::auth::secret_keys::repo::{self, ApiKeyDoc};
use crate::services::auth::tenants::repo::TenantDoc;
use crate::services::auth::users::repo::UserDoc;

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

#[utoipa::path(
    post,
    path = "/v1/auth/signup",
    tag = "Authentication",
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
    // Use new repos if available, fall back to old
    if let (Some(tenants_repo), Some(users_repo)) = (&state.tenants_repo, &state.users_repo) {
        return signup_v2(&state, tenants_repo.as_ref(), users_repo.as_ref(), body).await;
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

    if let Err(e) = send_verification_email(
        &state.config.resend_api_key,
        &state.config.resend_from_email,
        &email,
        &verify_url,
        Some(&full_key),
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

/// New signup flow: creates Tenant + User, defers key generation to verify.
async fn signup_v2(
    state: &AppState,
    tenants_repo: &dyn crate::services::auth::tenants::repo::TenantsRepository,
    users_repo: &dyn crate::services::auth::users::repo::UsersRepository,
    body: SignupRequest,
) -> Response {
    let email = body.email.trim().to_lowercase();

    if !email.contains('@') || email.len() < 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid email address", "code": "invalid_email" })),
        )
            .into_response();
    }

    // Check if email already exists in new users collection
    if let Ok(Some(existing)) = users_repo.find_by_email(&email).await {
        if existing.verified {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Email already registered. Use key rotation to get a new key, or contact support.",
                    "code": "email_exists"
                })),
            )
                .into_response();
        }
        // Unverified — allow re-signup (fall through to upsert)
    }

    // Also check old auth_repo for existing verified accounts
    if let Some(auth_repo) = &state.auth_repo {
        if let Some(existing) = auth_repo.find_key_by_email(&email).await {
            if existing.verified {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({
                        "error": "Email already registered. Use key rotation to get a new key, or contact support.",
                        "code": "email_exists"
                    })),
                )
                    .into_response();
            }
        }
    }

    let verify_token = keys::generate_verify_token();
    let expires_at = mongodb::bson::DateTime::from_millis(
        chrono::Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000,
    );

    // Create tenant
    let tenant_id = ObjectId::new();
    let tenant_doc = TenantDoc {
        id: Some(tenant_id),
        monthly_quota: 100,
        created_at: mongodb::bson::DateTime::now(),
    };

    if let Err(e) = tenants_repo.create(&tenant_doc).await {
        tracing::error!("Failed to create tenant: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    // Create or update user
    let user_doc = UserDoc {
        id: Some(ObjectId::new()),
        tenant_id,
        email: email.clone(),
        verified: false,
        is_owner: true,
        verify_token: Some(verify_token.clone()),
        verify_token_expires_at: Some(expires_at),
        created_at: mongodb::bson::DateTime::now(),
    };

    if let Err(e) = users_repo.upsert_by_email(&user_doc).await {
        tracing::error!("Failed to upsert user: {e}");
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

    if let Err(e) = send_verification_email(
        &state.config.resend_api_key,
        &state.config.resend_from_email,
        &email,
        &verify_url,
        None, // No key in email — shown on verify
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

    tracing::info!(email = %email, "Signup v2: verification email sent (key deferred to verify)");

    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Verification email sent. Click the link to verify your email and receive your API key.",
            "note": "Your API key will be shown once after verification. Save it — we can't show it again."
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/auth/verify",
    tag = "Authentication",
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
    // Try new users repo first
    if let (Some(users_repo), Some(secret_keys_repo)) = (&state.users_repo, &state.secret_keys_repo)
    {
        return verify_email_v2(
            users_repo.as_ref(),
            secret_keys_repo.as_ref(),
            &params.token,
        )
        .await;
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

/// New verify flow: validates token, generates key for owners, returns key in JSON.
async fn verify_email_v2(
    users_repo: &dyn crate::services::auth::users::repo::UsersRepository,
    secret_keys_repo: &dyn crate::services::auth::secret_keys::new_repo::SecretKeysRepository,
    token: &str,
) -> Response {
    let user = match users_repo.verify_user(token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid or expired verification token",
                    "code": "invalid_token"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to verify user: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    if user.is_owner {
        // Owner verification: generate first key
        let (full_key, key_hash, key_prefix) = keys::generate_api_key();
        let user_id = user.id.unwrap_or_else(ObjectId::new);

        let key_doc = SecretKeyDoc {
            id: ObjectId::new(),
            tenant_id: user.tenant_id,
            created_by: user_id,
            key_hash,
            key_prefix: key_prefix.clone(),
            created_at: mongodb::bson::DateTime::now(),
        };

        if let Err(e) = secret_keys_repo.create_key(&key_doc).await {
            tracing::error!("Failed to create secret key: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }

        tracing::info!(tenant_id = %user.tenant_id, "Owner verified, key created");

        (
            StatusCode::OK,
            Json(json!({
                "message": "Email verified! Your API key is below. Save it — we can't show it again.",
                "code": "verified",
                "key": full_key,
                "key_prefix": key_prefix,
            })),
        )
            .into_response()
    } else {
        // Invited user verification: no key generated
        tracing::info!(tenant_id = %user.tenant_id, email = %user.email, "Invited user verified");

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

async fn send_verification_email(
    resend_api_key: &str,
    from_email: &str,
    to: &str,
    verify_url: &str,
    api_key: Option<&str>,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let key_block = if let Some(key) = api_key {
        format!(
            r#"<p style="margin-top: 24px;">Your API key:</p>
                <code style="display: block; padding: 12px; background: #f4f4f5; border-radius: 6px; word-break: break-all; font-size: 14px;">{key}</code>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">Save this key — we can't show it again. The key won't work until you verify your email.</p>"#
        )
    } else {
        r#"<p style="color: #71717a; font-size: 13px; margin-top: 24px;">Your API key will be shown once after verification. Save it — we can't show it again.</p>"#.to_string()
    };

    let body = json!({
        "from": from_email,
        "to": [to],
        "subject": "Verify your Rift API key",
        "html": format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">Verify your email</h2>
                <p>Click the button below to activate your Rift API key:</p>
                <a href="{verify_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Verify Email</a>
                {key_block}
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
        ),
    });

    let resp = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {resend_api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Resend API error {status}: {text}"))
    }
}
