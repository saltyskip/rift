use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Serialize)]
pub struct CreateDomainRequest {
    pub domain: String,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDomainResponse {
    pub domain: String,
    pub verified: bool,
    pub verification_token: String,
    pub txt_record: String,
    pub cname_target: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainDetail {
    pub domain: String,
    pub verified: bool,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListDomainsResponse {
    pub domains: Vec<DomainDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyDomainResponse {
    pub domain: String,
    pub verified: bool,
    #[serde(default)]
    pub tls: String,
}

impl RiftClient {
    pub async fn create_domain(
        &self,
        request: &CreateDomainRequest,
    ) -> Result<CreateDomainResponse, RiftClientError> {
        self.post("/v1/domains", request, false).await
    }

    pub async fn list_domains(&self) -> Result<ListDomainsResponse, RiftClientError> {
        self.get("/v1/domains").await
    }

    pub async fn verify_domain(
        &self,
        domain: &str,
    ) -> Result<VerifyDomainResponse, RiftClientError> {
        self.post::<(), VerifyDomainResponse>(&format!("/v1/domains/{domain}/verify"), &(), false)
            .await
    }
}
