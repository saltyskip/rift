use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::ClientConfig;
use crate::credentials::ClientCredentials;
use crate::error::RiftClientError;

#[derive(Debug, serde::Deserialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Clone)]
pub struct RiftClient {
    pub(crate) base_url: String,
    pub(crate) http: reqwest::Client,
    pub(crate) credentials: Option<ClientCredentials>,
}

impl RiftClient {
    pub fn new(config: ClientConfig, credentials: Option<ClientCredentials>) -> Self {
        Self {
            base_url: config.base_url,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            credentials,
        }
    }

    pub fn with_secret_key(secret_key: String, base_url: Option<String>) -> Self {
        Self::new(
            ClientConfig {
                base_url: base_url
                    .unwrap_or_default()
                    .if_empty("https://api.riftl.ink"),
            },
            Some(ClientCredentials::SecretKey(secret_key)),
        )
    }

    pub fn with_publishable_key(publishable_key: String, base_url: Option<String>) -> Self {
        Self::new(
            ClientConfig {
                base_url: base_url
                    .unwrap_or_default()
                    .if_empty("https://api.riftl.ink"),
            },
            Some(ClientCredentials::PublishableKey(publishable_key)),
        )
    }

    pub fn anonymous(base_url: Option<String>) -> Self {
        Self::new(
            ClientConfig {
                base_url: base_url
                    .unwrap_or_default()
                    .if_empty("https://api.riftl.ink"),
            },
            None,
        )
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, RiftClientError> {
        let request = self.apply_auth(self.http.get(self.url(path)), false);
        self.send(request).await
    }

    #[allow(dead_code)]
    pub(crate) async fn delete_empty(&self, path: &str) -> Result<(), RiftClientError> {
        let request = self.apply_auth(self.http.delete(self.url(path)), false);
        self.send_empty(request).await
    }

    pub(crate) async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        publishable_via_query: bool,
    ) -> Result<T, RiftClientError> {
        let request = self
            .apply_auth(self.http.post(self.url(path)), publishable_via_query)
            .json(body);
        self.send(request).await
    }

    pub(crate) async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, RiftClientError> {
        let request = self
            .apply_auth(self.http.put(self.url(path)), false)
            .json(body);
        self.send(request).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn apply_auth(
        &self,
        builder: reqwest::RequestBuilder,
        publishable_via_query: bool,
    ) -> reqwest::RequestBuilder {
        match &self.credentials {
            Some(ClientCredentials::SecretKey(key)) => {
                builder.header("Authorization", format!("Bearer {key}"))
            }
            Some(ClientCredentials::PublishableKey(key)) if publishable_via_query => {
                builder.query(&[("key", key)])
            }
            Some(ClientCredentials::PublishableKey(key)) => {
                builder.header("Authorization", format!("Bearer {key}"))
            }
            None => builder,
        }
    }

    async fn send<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, RiftClientError> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body: ApiErrorBody = response.json().await.unwrap_or(ApiErrorBody {
                error: "Unknown error".to_string(),
            });
            return Err(RiftClientError::Api {
                status,
                message: body.error,
            });
        }
        response
            .json::<T>()
            .await
            .map_err(|e| RiftClientError::Deserialize(e.to_string()))
    }

    #[allow(dead_code)]
    async fn send_empty(&self, request: reqwest::RequestBuilder) -> Result<(), RiftClientError> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body: ApiErrorBody = response.json().await.unwrap_or(ApiErrorBody {
                error: "Unknown error".to_string(),
            });
            return Err(RiftClientError::Api {
                status,
                message: body.error,
            });
        }
        Ok(())
    }
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
