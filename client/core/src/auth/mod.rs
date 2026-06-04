use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Serialize)]
pub struct SignupRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct SignupResponse {
    pub message: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct CreatePublishableKeyRequest {
    pub domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishableKeyDetail {
    pub id: String,
    pub key_prefix: String,
    pub domain: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPublishableKeysResponse {
    pub keys: Vec<PublishableKeyDetail>,
}

#[derive(Debug, Deserialize)]
pub struct UserDetail {
    pub id: String,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserDetail>,
}

#[derive(Debug, Serialize)]
pub struct InviteUserRequest {
    pub email: String,
}

/// `GET /v1/auth/me` response — the resolved user + tenant for the active
/// session. Used after a browser login to confirm and display who you are.
#[derive(Debug, Deserialize)]
pub struct MeResponse {
    pub user: MeUser,
    pub tenant: MeTenant,
}

#[derive(Debug, Deserialize)]
pub struct MeUser {
    pub id: String,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
}

#[derive(Debug, Deserialize)]
pub struct MeTenant {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct InviteUserResponse {
    pub id: String,
    pub email: String,
    pub status: String,
}

impl RiftClient {
    pub async fn signup(
        &self,
        email: impl Into<String>,
    ) -> Result<SignupResponse, RiftClientError> {
        self.post(
            "/v1/auth/signup",
            &SignupRequest {
                email: email.into(),
            },
            false,
        )
        .await
    }

    pub async fn list_publishable_keys(
        &self,
    ) -> Result<ListPublishableKeysResponse, RiftClientError> {
        self.get("/v1/auth/publishable-keys").await
    }

    pub async fn list_users(&self) -> Result<ListUsersResponse, RiftClientError> {
        self.get("/v1/auth/users").await
    }

    pub async fn invite_user(
        &self,
        email: impl Into<String>,
    ) -> Result<InviteUserResponse, RiftClientError> {
        self.post(
            "/v1/auth/users",
            &InviteUserRequest {
                email: email.into(),
            },
            false,
        )
        .await
    }

    pub async fn remove_user(&self, user_id: &str) -> Result<(), RiftClientError> {
        self.delete_empty(&format!("/v1/auth/users/{user_id}"))
            .await
    }

    /// Resolve the active session to its user + tenant. Confirms a session
    /// token works (used after browser login and by `rift whoami`).
    pub async fn me(&self) -> Result<MeResponse, RiftClientError> {
        self.get("/v1/auth/me").await
    }

    /// Revoke the active session server-side (called on `rift logout` when
    /// logged in via a session).
    pub async fn signout(&self) -> Result<(), RiftClientError> {
        self.post_empty("/v1/auth/signout").await
    }
}
