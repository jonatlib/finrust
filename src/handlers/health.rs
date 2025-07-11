use axum::{extract::State, http::StatusCode, response::Json};
use tracing::instrument;
use crate::schemas::{AppState, HealthResponse};

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 500, description = "Service is unhealthy", body = crate::schemas::ErrorResponse)
    )
)]
#[instrument]
pub async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    // Test database connection
    let db_status = match state.db.ping().await {
        Ok(_) => "connected".to_string(),
        Err(_) => "disconnected".to_string(),
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status,
    };

    Ok(Json(response))
}