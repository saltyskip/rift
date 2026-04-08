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
}
