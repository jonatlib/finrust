use crate::schemas::{AppState, HealthResponse};
use axum::{extract::State, http::StatusCode, response::Json};
use tracing::{instrument, error, warn, info, debug, trace};

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
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    trace!("Entering health_check function");
    debug!("Performing health check");

    // Test database connection
    trace!("Testing database connection");
    let db_status = match state.db.ping().await {
        Ok(_) => {
            debug!("Database connection is healthy");
            "connected".to_string()
        }
        Err(db_error) => {
            warn!("Database connection failed during health check: {}", db_error);
            "disconnected".to_string()
        }
    };

    let version = env!("CARGO_PKG_VERSION");
    let is_healthy = db_status == "connected";

    if is_healthy {
        debug!("Health check passed - all systems operational");
    } else {
        warn!("Health check failed - database is disconnected");
    }

    let response = HealthResponse {
        status: if is_healthy { "healthy" } else { "unhealthy" }.to_string(),
        version: version.to_string(),
        database: db_status,
    };

    info!("Health check completed - status: {}, database: {}, version: {}", 
          response.status, response.database, response.version);

    if is_healthy {
        Ok(Json(response))
    } else {
        error!("Health check failed - returning 500 status");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
