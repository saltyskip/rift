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
use crate::services::auth::users::service::UserError;

// ── Request / Response types ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteUserRequest {
    /// Email address of the user to invite.
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InviteUserResponse {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "alice@example.com")]
    pub email: String,
    #[schema(example = "verification_sent")]
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "alice@example.com")]
    pub email: String,
    #[schema(example = true)]
    pub verified: bool,
    #[schema(example = false)]
    pub is_owner: bool,
    #[schema(example = "2025-06-15T10:30:00Z")]
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
    tag = "Team Members",
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
    let Some(svc) = &state.users_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    // Log-only quota check (Phase A-1).
    if let Some(ref quota) = state.quota_service {
        if let Err(e) = quota
            .check(
                &tenant.0,
                crate::services::billing::quota::Resource::InviteTeamMember,
            )
            .await
        {
            tracing::warn!(error = %e, "quota_check_invite_team_member_error");
        }
    }

    match svc
        .invite(
            tenant.0,
            &req.email,
            &state.config.public_url,
            &state.config.resend_api_key,
            &state.config.resend_from_email,
        )
        .await
    {
        Ok(result) => {
            tracing::info!(email = %result.email, tenant_id = %tenant.0, "User invited");
            (
                StatusCode::CREATED,
                Json(json!(InviteUserResponse {
                    id: result.user_id.to_hex(),
                    email: result.email,
                    status: "verification_sent".to_string(),
                })),
            )
                .into_response()
        }
        Err(e) => error_response(&e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/auth/users",
    tag = "Team Members",
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
    let Some(svc) = &state.users_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match svc.list(&tenant.0).await {
        Ok(users) => {
            let details: Vec<UserDetail> = users
                .iter()
                .map(|u| UserDetail {
                    id: u.id.to_hex(),
                    email: u.email.clone(),
                    verified: u.verified,
                    is_owner: u.is_owner,
                    created_at: u.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!(ListUsersResponse { users: details })).into_response()
        }
        Err(e) => error_response(&e),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/auth/users/{user_id}",
    tag = "Team Members",
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
    let Some(svc) = &state.users_service else {
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

    match svc.delete(tenant.0, oid).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

// ── Error response helper ──

fn error_response(e: &UserError) -> Response {
    let status = match e {
        UserError::InvalidEmail => StatusCode::BAD_REQUEST,
        UserError::EmailExists | UserError::UserExists | UserError::LastUser => {
            StatusCode::CONFLICT
        }
        UserError::NotFound => StatusCode::NOT_FOUND,
        UserError::EmailFailed(_) | UserError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({ "error": e.to_string(), "code": e.code() })),
    )
        .into_response()
}
