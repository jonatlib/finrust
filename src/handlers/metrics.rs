use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use common::metrics::{AccountMetricsDto, DashboardMetricsDto};
use compute::account::AccountStateCalculator;
use compute::default_compute;
use compute::metrics::{account_metrics, cross_account_metrics};
use model::entities::account;
use sea_orm::EntityTrait;
use tracing::{debug, error, info, instrument, trace, warn};

/// Get the full financial dashboard with cross-account and per-account metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics/dashboard",
    tag = "metrics",
    responses(
        (status = 200, description = "Dashboard metrics retrieved successfully", body = ApiResponse<DashboardMetricsDto>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_dashboard_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardMetricsDto>>, StatusCode> {
    trace!("Entering get_dashboard_metrics");
    let today = chrono::Utc::now().date_naive();
    let compute = default_compute(None);

    debug!("Computing dashboard metrics for date: {}", today);
    match cross_account_metrics::compute_dashboard_metrics(
        &compute as &dyn AccountStateCalculator,
        &state.db,
        today,
    )
        .await
    {
        Ok(dashboard) => {
            info!(
                total_net_worth = %dashboard.total_net_worth,
                account_count = dashboard.account_metrics.len(),
                "Dashboard metrics computed successfully"
            );
            Ok(Json(ApiResponse {
                data: dashboard,
                message: "Dashboard metrics retrieved successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to compute dashboard metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get detailed metrics for a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/metrics",
    tag = "metrics",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account metrics retrieved successfully", body = ApiResponse<AccountMetricsDto>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_metrics(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountMetricsDto>>, StatusCode> {
    trace!("Entering get_account_metrics for account_id: {}", account_id);

    let account_model = match account::Entity::find_by_id(account_id)
        .one(&state.db)
        .await
    {
        Ok(Some(account)) => {
            debug!("Found account: {}", account.name);
            account
        }
        Ok(None) => {
            warn!("Account with ID {} not found", account_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(db_error) => {
            error!(
                "Failed to retrieve account with ID {}: {}",
                account_id, db_error
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !account_model.include_in_statistics {
        warn!(
            "Account with ID {} is not included in statistics",
            account_id
        );
        return Err(StatusCode::NOT_FOUND);
    }

    let today = chrono::Utc::now().date_naive();
    let compute = default_compute(None);

    match account_metrics::compute_account_metrics(
        &compute as &dyn AccountStateCalculator,
        &state.db,
        &account_model,
        today,
    )
        .await
    {
        Ok(metrics) => {
            info!(
                account_id,
                balance = %metrics.current_balance,
                "Account metrics computed successfully"
            );
            Ok(Json(ApiResponse {
                data: metrics,
                message: "Account metrics retrieved successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!(
                "Failed to compute metrics for account ID {}: {}",
                account_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
