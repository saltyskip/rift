use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiftError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Deserialization error: {0}")]
    Deserialize(String),
}
