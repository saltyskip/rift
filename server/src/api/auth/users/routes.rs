use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::auth::middleware::TenantId;
use crate::app::AppState;
use crate::services::auth::keys;
use crate::services::auth::users::repo::UserDoc;

// ── Request / Response types ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteUserRequest {
    /// Email address of the user to invite.
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InviteUserResponse {
    pub id: String,
    pub email: String,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDetail {
    pub id: String,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListUsersResponse {
    pub users: Vec<UserDetail>,
}

// ── Handlers ──

#[utoipa::path(
    post,
    path = "/v1/auth/users",
    tag = "Authentication",
    request_body = InviteUserRequest,
    responses(
        (status = 201, description = "Invitation sent", body = InviteUserResponse),
        (status = 400, description = "Invalid email", body = crate::error::ErrorResponse),
        (status = 409, description = "User already on this tenant", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn invite_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<InviteUserRequest>,
) -> Response {
    let Some(users_repo) = &state.users_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let email = req.email.trim().to_lowercase();

    if !email.contains('@') || email.len() < 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid email address", "code": "invalid_email" })),
        )
            .into_response();
    }

    // Check if already on this tenant
    if let Ok(Some(_)) = users_repo.find_by_tenant_and_email(&tenant.0, &email).await {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "User already exists on this team", "code": "user_exists" })),
        )
            .into_response();
    }

    let verify_token = keys::generate_verify_token();
    let expires_at = mongodb::bson::DateTime::from_millis(
        chrono::Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000,
    );

    let user_id = ObjectId::new();
    let user_doc = UserDoc {
        id: Some(user_id),
        tenant_id: tenant.0,
        email: email.clone(),
        verified: false,
        is_owner: false,
        verify_token: Some(verify_token.clone()),
        verify_token_expires_at: Some(expires_at),
        created_at: mongodb::bson::DateTime::now(),
    };

    if let Err(e) = users_repo.create(&user_doc).await {
        tracing::error!("Failed to create invited user: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Internal error", "code": "db_error" })),
        )
            .into_response();
    }

    // Send verification email
    let verify_url = format!(
        "{}/v1/auth/verify?token={verify_token}",
        state.config.public_url
    );

    if let Err(e) = send_invite_email(
        &state.config.resend_api_key,
        &state.config.resend_from_email,
        &email,
        &verify_url,
    )
    .await
    {
        tracing::error!("Failed to send invite email: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to send invitation email", "code": "email_error" })),
        )
            .into_response();
    }

    tracing::info!(email = %email, tenant_id = %tenant.0, "User invited");

    (
        StatusCode::CREATED,
        Json(json!(InviteUserResponse {
            id: user_id.to_hex(),
            email,
            status: "verification_sent".to_string(),
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/auth/users",
    tag = "Authentication",
    responses(
        (status = 200, description = "List of users", body = ListUsersResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(users_repo) = &state.users_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match users_repo.list_by_tenant(&tenant.0).await {
        Ok(docs) => {
            let users: Vec<UserDetail> = docs
                .iter()
                .map(|d| UserDetail {
                    id: d.id.map(|id| id.to_hex()).unwrap_or_default(),
                    email: d.email.clone(),
                    verified: d.verified,
                    is_owner: d.is_owner,
                    created_at: d.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!(ListUsersResponse { users })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list users: {e}");
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
    path = "/v1/auth/users/{user_id}",
    tag = "Authentication",
    params(("user_id" = String, Path, description = "User ID")),
    responses(
        (status = 204, description = "User removed"),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Cannot remove last verified user", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(user_id): Path<String>,
) -> Response {
    let Some(users_repo) = &state.users_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Ok(oid) = ObjectId::parse_str(&user_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid user ID", "code": "bad_request" })),
        )
            .into_response();
    };

    // Guard: can't remove last verified user
    match users_repo.count_verified_by_tenant(&tenant.0).await {
        Ok(count) if count <= 1 => {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Cannot remove the last verified user on this team",
                    "code": "last_user"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to count users: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
        _ => {}
    }

    match users_repo.delete(&tenant.0, &oid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "User not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete user: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── Email ──

async fn send_invite_email(
    resend_api_key: &str,
    from_email: &str,
    to: &str,
    verify_url: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "from": from_email,
        "to": [to],
        "subject": "You've been invited to a Rift team",
        "html": format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">You've been invited</h2>
                <p>Click the button below to join the team on Rift:</p>
                <a href="{verify_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Accept Invitation</a>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">This link expires in 24 hours.</p>
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
