use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Serialize)]
pub struct CreateAppRequest {
    pub platform: String,
    pub bundle_id: Option<String>,
    pub team_id: Option<String>,
    pub package_name: Option<String>,
    pub sha256_fingerprints: Option<Vec<String>>,
    pub app_name: Option<String>,
    pub icon_url: Option<String>,
    pub theme_color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppDetail {
    pub id: String,
    pub platform: String,
    pub bundle_id: Option<String>,
    pub team_id: Option<String>,
    pub package_name: Option<String>,
    pub sha256_fingerprints: Option<Vec<String>>,
    pub app_name: Option<String>,
    pub icon_url: Option<String>,
    pub theme_color: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListAppsResponse {
    pub apps: Vec<AppDetail>,
}

impl RiftClient {
    pub async fn create_app(
        &self,
        request: &CreateAppRequest,
    ) -> Result<AppDetail, RiftClientError> {
        self.post("/v1/apps", request, false).await
    }

    pub async fn list_apps(&self) -> Result<ListAppsResponse, RiftClientError> {
        self.get("/v1/apps").await
    }
}
