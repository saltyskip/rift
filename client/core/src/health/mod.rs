use serde::Deserialize;

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Deserialize)]
pub struct HealthInfo {
    pub title: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub info: HealthInfo,
}

impl RiftClient {
    pub async fn health(&self) -> Result<HealthResponse, RiftClientError> {
        self.get("/health").await
    }
}
