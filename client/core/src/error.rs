use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiftClientError {
    #[error("network error: {0}")]
    Network(String),

    #[error("api error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("deserialization error: {0}")]
    Deserialize(String),
}

impl From<reqwest::Error> for RiftClientError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value.to_string())
    }
}
