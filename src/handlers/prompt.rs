use crate::cli::commands::generate_prompt::build_prompt;
use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{error, info, instrument};
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PromptQuery {
    /// Number of months of historical data to include (default: 24)
    #[serde(default = "default_months")]
    pub months: u32,
}

fn default_months() -> u32 {
    24
}

/// Generate a financial assessment prompt for an external LLM
#[utoipa::path(
    get,
    path = "/api/v1/prompt",
    params(PromptQuery),
    responses(
        (status = 200, description = "Financial assessment prompt generated", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "prompt"
)]
#[instrument(skip(state))]
pub async fn get_financial_prompt(
    State(state): State<AppState>,
    Query(query): Query<PromptQuery>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    info!("Generating financial assessment prompt via API (months={})", query.months);

    match build_prompt(&state.db, query.months).await {
        Ok(prompt) => Ok(Json(ApiResponse {
            data: prompt,
            message: "Success".to_string(),
            success: true,
        })),
        Err(e) => {
            error!("Failed to generate prompt: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to generate prompt: {}", e),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}
