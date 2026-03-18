use crate::schemas::{ApiResponse, AppState};
use axum::{extract::State, http::StatusCode, response::Json};
use tracing::{info, instrument};

/// Flush all cached data, forcing fresh computation on next request
#[instrument]
pub async fn flush_cache(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    state.cache.invalidate_all();
    info!("Cache flushed by explicit request");
    Ok(Json(ApiResponse {
        data: "Cache flushed".to_string(),
        message: "All cached data has been invalidated".to_string(),
        success: true,
    }))
}
