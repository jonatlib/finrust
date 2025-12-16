use crate::helpers::converters::convert_dataframe_to_timeseries;
use crate::schemas::{ApiResponse, AppState, CachedData, TimeseriesQuery, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use common::AccountStateTimeseries;
use compute::{account::AccountStateCalculator, default_compute, default_compute_with_scenario};
use model::entities::account;
use sea_orm::EntityTrait;
use tracing::{instrument, error, warn, info, debug, trace};

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
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_timeseries(
    Path(account_id): Path<i32>,
    Valid(Query(query)): Valid<Query<TimeseriesQuery>>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStateTimeseries>>, StatusCode> {
    trace!("Entering get_account_timeseries function for account_id: {}", account_id);
    debug!("Fetching timeseries for account ID: {} with query: {:?}", account_id, query);

    // Create cache key
    let cache_key = format!("timeseries_{}_{:?}", account_id, query);
    trace!("Generated cache key: {}", cache_key);

    // Check cache first
    debug!("Checking cache for timeseries");
    if let Some(CachedData::Timeseries(timeseries)) = state.cache.get(&cache_key).await {
        info!("Timeseries for account ID {} retrieved from cache", account_id);
        let response = ApiResponse {
            data: timeseries,
            message: "Account timeseries retrieved from cache".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }
    debug!("Cache miss for account timeseries, proceeding with database query");

    // Get the account from database
    trace!("Looking up account with ID: {}", account_id);
    let account_model = match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account)) => {
            debug!("Found account: {}", account.name);
            account
        }
        Ok(None) => {
            warn!("Account with ID {} not found", account_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(db_error) => {
            error!("Failed to retrieve account with ID {}: {}", account_id, db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Only include accounts that are marked for statistics (unless include_ignored is true)
    if !query.include_ignored && !account_model.include_in_statistics {
        warn!("Account with ID {} is not included in statistics and include_ignored=false", account_id);
        return Err(StatusCode::NOT_FOUND);
    }

    // Compute timeseries using the compute module
    debug!("Computing timeseries for account: {} from {} to {}", 
           account_model.name, query.start_date, query.end_date);
    let accounts = vec![account_model];
    let compute = default_compute(None);

    trace!("Executing timeseries computation");
    let timeseries_result = compute
        .compute_account_state(&state.db, &accounts, query.start_date, query.end_date)
        .await;

    let timeseries = match timeseries_result {
        Ok(df) => {
            debug!("Successfully computed timeseries dataframe, converting to timeseries format");
            // Convert DataFrame to AccountStateTimeseries manually
            match convert_dataframe_to_timeseries(df) {
                Ok(timeseries) => {
                    debug!("Successfully converted dataframe to timeseries");
                    timeseries
                }
                Err(conversion_error) => {
                    error!("Failed to convert dataframe to timeseries for account ID {}: {}", 
                           account_id, conversion_error);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(compute_error) => {
            error!("Failed to compute timeseries for account ID {}: {}", account_id, compute_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Cache the result
    trace!("Caching timeseries result with key: {}", cache_key);
    state
        .cache
        .insert(cache_key, CachedData::Timeseries(timeseries.clone()))
        .await;
    debug!("Timeseries cached successfully");

    info!("Account timeseries for ID {} retrieved and cached successfully", account_id);
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
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_all_accounts_timeseries(
    Valid(Query(query)): Valid<Query<TimeseriesQuery>>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStateTimeseries>>, StatusCode> {
    trace!("Entering get_all_accounts_timeseries function");
    debug!("Fetching timeseries for all accounts with query: {:?}", query);

    // Get all accounts that are included in statistics
    trace!("Querying database for all accounts");
    let accounts = match account::Entity::find().all(&state.db).await {
        Ok(accounts) => {
            let all_count = accounts.len();
            let filtered_accounts: Vec<_> = accounts
                .into_iter()
                .filter(|a| a.include_in_statistics)
                .collect();
            let filtered_count = filtered_accounts.len();
            debug!("Retrieved {} total accounts, {} included in statistics", all_count, filtered_count);
            filtered_accounts
        }
        Err(db_error) => {
            error!("Failed to retrieve accounts from database: {}", db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if accounts.is_empty() {
        warn!("No accounts found that are included in statistics");
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
    debug!("Computing timeseries for {} accounts from {} to {} (scenario_id={:?})",
           accounts.len(), query.start_date, query.end_date, query.scenario_id);
    let compute = default_compute_with_scenario(None, query.scenario_id);

    trace!("Executing timeseries computation for all accounts");
    let timeseries_result = compute
        .compute_account_state(&state.db, &accounts, query.start_date, query.end_date)
        .await;

    let timeseries = match timeseries_result {
        Ok(df) => {
            debug!("Successfully computed timeseries dataframe for all accounts, converting to timeseries format");
            // Convert DataFrame to AccountStateTimeseries using the conversion function
            match convert_dataframe_to_timeseries(df) {
                Ok(timeseries) => {
                    debug!("Successfully converted dataframe to timeseries for all accounts");
                    timeseries
                }
                Err(conversion_error) => {
                    error!("Failed to convert dataframe to timeseries for all accounts: {}", conversion_error);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(compute_error) => {
            error!("Failed to compute timeseries for all accounts: {}", compute_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Successfully computed timeseries for {} accounts", accounts.len());
    let response = ApiResponse {
        data: timeseries,
        message: "All accounts timeseries retrieved successfully".to_string(),
        success: true,
    };

    Ok(Json(response))
}
