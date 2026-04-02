use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

use axum::extract::Path;

use crate::api::auth::middleware::{AuthKeyId, TenantId};
use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::new_repo::{SecretKeyCreateRequestDoc, SecretKeyDoc};
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

// ── Secret Key CRUD handlers ──

#[utoipa::path(
    post,
    path = "/v1/auth/secret-keys",
    tag = "Authentication",
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
    let (Some(users_repo), Some(sk_repo)) = (&state.users_repo, &state.secret_keys_repo) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let email = req.email.trim().to_lowercase();

    // Permission check: email must be a verified user on this tenant
    let user = match users_repo.find_by_tenant_and_email(&tenant.0, &email).await {
        Ok(Some(u)) if u.verified => u,
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "User has not verified their email", "code": "user_unverified" })),
            )
                .into_response();
        }
        Ok(None) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Email is not a member of this team", "code": "not_a_member" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to look up user: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    let user_id = user.id.unwrap_or_else(ObjectId::new);

    // Max 5 keys per tenant
    match sk_repo.count_by_tenant(&tenant.0).await {
        Ok(count) if count >= 5 => {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "Maximum of 5 secret keys per team", "code": "key_limit" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to count keys: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
        _ => {}
    }

    // Cooldown: reject if pending request exists
    if let Ok(Some(_)) = sk_repo.find_pending_request(&tenant.0, &user_id).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "A key creation request is already pending. Check your email or wait 15 minutes.",
                "code": "request_pending"
            })),
        )
            .into_response();
    }

    // Generate code, hash it, store request
    let code = keys::generate_key_create_code();
    let token_hash = keys::hash_key(&code);
    let expires_at = mongodb::bson::DateTime::from_millis(
        chrono::Utc::now().timestamp_millis() + 15 * 60 * 1000,
    );

    let request_doc = SecretKeyCreateRequestDoc {
        id: None,
        tenant_id: tenant.0,
        user_id,
        token_hash,
        attempts: 0,
        expires_at,
        created_at: mongodb::bson::DateTime::now(),
    };

    if let Err(e) = sk_repo.create_request(&request_doc).await {
        tracing::error!("Failed to create key request: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    // Send email with code
    if let Err(e) = send_key_create_email(
        &state.config.resend_api_key,
        &state.config.resend_from_email,
        &email,
        &code,
    )
    .await
    {
        tracing::error!("Failed to send key creation email: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to send confirmation email", "code": "email_error" })),
        )
            .into_response();
    }

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

#[utoipa::path(
    post,
    path = "/v1/auth/secret-keys/confirm",
    tag = "Authentication",
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
    let (Some(users_repo), Some(sk_repo)) = (&state.users_repo, &state.secret_keys_repo) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let email = req.email.trim().to_lowercase();

    // Look up the user
    let user = match users_repo.find_by_tenant_and_email(&tenant.0, &email).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid request", "code": "bad_request" })),
            )
                .into_response();
        }
    };

    let user_id = user.id.unwrap_or_else(ObjectId::new);

    // Rate limit: increment attempts first
    match sk_repo
        .increment_request_attempts(&tenant.0, &user_id)
        .await
    {
        Ok(attempts) if attempts > 5 => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Too many attempts. Request a new code.",
                    "code": "too_many_attempts"
                })),
            )
                .into_response();
        }
        Ok(0) => {
            // No pending request found
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "No pending key creation request. Request a new code first.",
                    "code": "no_pending_request"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to increment attempts: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
        _ => {}
    }

    // Validate token (hash and compare)
    let token_hash = keys::hash_key(&req.token.trim().to_uppercase());
    match sk_repo
        .validate_and_consume_request(&tenant.0, &user_id, &token_hash)
        .await
    {
        Ok(true) => {} // Token valid, request consumed
        Ok(false) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid or expired confirmation code",
                    "code": "invalid_code"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to validate request: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    }

    // Generate key and create
    let (full_key, key_hash, key_prefix) = keys::generate_api_key();
    let key_id = ObjectId::new();
    let now = mongodb::bson::DateTime::now();

    let key_doc = SecretKeyDoc {
        id: key_id,
        tenant_id: tenant.0,
        created_by: user_id,
        key_hash,
        key_prefix: key_prefix.clone(),
        created_at: now,
    };

    if let Err(e) = sk_repo.create_key(&key_doc).await {
        tracing::error!("Failed to create secret key: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    tracing::info!(tenant_id = %tenant.0, "New secret key created via email confirmation");

    (
        StatusCode::CREATED,
        Json(json!(CreateKeyResponse {
            id: key_id.to_hex(),
            key: full_key,
            key_prefix,
            created_at: now.try_to_rfc3339_string().unwrap_or_default(),
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/auth/secret-keys",
    tag = "Authentication",
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
    let Some(sk_repo) = &state.secret_keys_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match sk_repo.list_by_tenant(&tenant.0).await {
        Ok(docs) => {
            let keys: Vec<SecretKeyDetail> = docs
                .iter()
                .map(|d| SecretKeyDetail {
                    id: d.id.to_hex(),
                    key_prefix: d.key_prefix.clone(),
                    created_by: d.created_by.to_hex(),
                    created_at: d.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!(ListSecretKeysResponse { keys })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list secret keys: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/v1/auth/secret-keys/{key_id}",
    tag = "Authentication",
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
    let Some(sk_repo) = &state.secret_keys_repo else {
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

    // Guard: can't delete the key you're authenticated with
    if oid == auth_key.0 {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "Cannot delete the key you are currently authenticated with",
                "code": "self_delete"
            })),
        )
            .into_response();
    }

    // Guard: can't delete last key
    match sk_repo.count_by_tenant(&tenant.0).await {
        Ok(count) if count <= 1 => {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Cannot delete your only secret key",
                    "code": "last_key"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to count keys: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
        _ => {}
    }

    match sk_repo.delete_key(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Secret key not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete secret key: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── Emails ──

async fn send_key_create_email(
    resend_api_key: &str,
    from_email: &str,
    to: &str,
    code: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let body = json!({
        "from": from_email,
        "to": [to],
        "subject": format!("Your Rift key creation code: {code}"),
        "html": format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">Key creation confirmation</h2>
                <p>Use this code to confirm your new API key:</p>
                <code style="display: block; padding: 16px; background: #f4f4f5; border-radius: 6px; font-size: 24px; letter-spacing: 4px; text-align: center; margin: 20px 0;">{code}</code>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">This code expires in 15 minutes. If you didn't request this, you can safely ignore this email.</p>
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
