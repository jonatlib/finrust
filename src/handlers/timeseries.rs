use axum::{extract::{Path, Query, State}, http::StatusCode, response::Json};
use common::{AccountStateTimeseries};
use compute::{default_compute, account::AccountStateCalculator};
use model::entities::account;
use sea_orm::EntityTrait;
use tracing::instrument;
use crate::schemas::{AppState, ApiResponse, TimeseriesQuery, CachedData};
use crate::helpers::converters::convert_dataframe_to_timeseries;

/// Get timeseries data for a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/timeseries",
    tag = "timeseries",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account timeseries retrieved successfully", body = ApiResponse<AccountStateTimeseries>),
        (status = 404, description = "Account not found", body = crate::schemas::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::schemas::ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_timeseries(
    Path(account_id): Path<i32>,
    Query(query): Query<TimeseriesQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStateTimeseries>>, StatusCode> {
    // Create cache key
    let cache_key = format!("timeseries_{}_{:?}", account_id, query);

    // Check cache first
    if let Some(CachedData::Timeseries(timeseries)) = state.cache.get(&cache_key).await {
        let response = ApiResponse {
            data: timeseries,
            message: "Account timeseries retrieved from cache".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }

    // Get the account from database
    let account_model = match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account)) => account,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Only include accounts that are marked for statistics
    if !account_model.include_in_statistics {
        return Err(StatusCode::NOT_FOUND);
    }

    // Compute timeseries using the compute module
    let accounts = vec![account_model];
    let compute = default_compute(None);

    let timeseries_result = compute.compute_account_state(&state.db, &accounts, query.start_date, query.end_date).await;

    let timeseries = match timeseries_result {
        Ok(df) => {
            // Convert DataFrame to AccountStateTimeseries manually
            match convert_dataframe_to_timeseries(df) {
                Ok(timeseries) => timeseries,
                Err(_) => {
                    // Return error if conversion fails
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(_) => {
            // Return error if computation fails
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Cache the result
    state.cache.insert(cache_key, CachedData::Timeseries(timeseries.clone())).await;

    let response = ApiResponse {
        data: timeseries,
        message: "Account timeseries retrieved successfully".to_string(),
        success: true,
    };

    Ok(Json(response))
}

/// Get timeseries data for all accounts
#[utoipa::path(
    get,
    path = "/api/v1/accounts/timeseries",
    tag = "timeseries",
    responses(
        (status = 200, description = "All accounts timeseries retrieved successfully", body = ApiResponse<AccountStateTimeseries>),
        (status = 500, description = "Internal server error", body = crate::schemas::ErrorResponse)
    )
)]
#[instrument]
pub async fn get_all_accounts_timeseries(
    Query(query): Query<TimeseriesQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStateTimeseries>>, StatusCode> {
    // Get all accounts that are included in statistics
    let accounts = match account::Entity::find().all(&state.db).await {
        Ok(accounts) => accounts.into_iter().filter(|a| a.include_in_statistics).collect::<Vec<_>>(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if accounts.is_empty() {
        // Return empty timeseries when no accounts are found
        let empty_timeseries = AccountStateTimeseries::new(vec![]);
        let response = ApiResponse {
            data: empty_timeseries,
            message: "No accounts found for timeseries".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }

    // Compute timeseries for all accounts using the compute module
    let compute = default_compute(None);

    let timeseries_result = compute.compute_account_state(&state.db, &accounts, query.start_date, query.end_date).await;

    let timeseries = match timeseries_result {
        Ok(df) => {
            // Convert DataFrame to AccountStateTimeseries using the conversion function
            match convert_dataframe_to_timeseries(df) {
                Ok(timeseries) => timeseries,
                Err(_) => {
                    // Return error if conversion fails
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(_) => {
            // Return error if computation fails
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response = ApiResponse {
        data: timeseries,
        message: "All accounts timeseries retrieved successfully".to_string(),
        success: true,
    };

    Ok(Json(response))
}
