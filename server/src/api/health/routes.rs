use axum::Json;

use super::models::HealthResponse;

#[utoipa::path(
    get,
    path = "/health",
    tag = "System",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
    )
)]
#[tracing::instrument]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
