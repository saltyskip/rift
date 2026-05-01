use serde::Serialize;
use utoipa::ToSchema;

/// Error response returned on failures.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Human-readable error message.
    #[schema(example = "Not found: link with id 'abc123' does not exist")]
    pub error: String,
    /// Machine-readable error code.
    #[schema(example = "not_found")]
    pub code: String,
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}
