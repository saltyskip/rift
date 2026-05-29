//! Request / response DTOs for `api/auth/users/routes.rs`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteUserRequest {
    /// Email address of the user to invite.
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InviteUserResponse {
    pub id: crate::core::public_id::UserId,
    #[schema(example = "alice@example.com")]
    pub email: String,
    #[schema(example = "verification_sent")]
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDetail {
    pub id: crate::core::public_id::UserId,
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
