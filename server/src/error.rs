use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            AppError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        };

        (status, Json(body)).into_response()
    }
}
