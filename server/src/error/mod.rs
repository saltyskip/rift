pub mod models;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

pub use models::{AppError, ErrorResponse};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            AppError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let message = self.to_string();
        // sentry-tracing maps `error!` to a Sentry event and `warn!` to a
        // breadcrumb, so 5xx fires events while 4xx stays as breadcrumbs on
        // any subsequent error in the same request.
        if status.is_server_error() {
            tracing::error!(status = %status.as_u16(), code, %message, "request failed");
        } else {
            tracing::warn!(status = %status.as_u16(), code, %message, "request rejected");
        }

        let body = ErrorResponse {
            error: message,
            code: code.to_string(),
        };

        (status, Json(body)).into_response()
    }
}
