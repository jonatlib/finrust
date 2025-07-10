use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use chrono::{Datelike, NaiveDate};
use common::{
    AccountStatistics, AccountStatisticsCollection, AccountStateTimeseries, 
    TimePeriod, DateRange
};
use compute::{default_compute, account::AccountStateCalculator, account_stats};
use model::entities::account;
use moka::future::Cache;
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
};
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// Application state shared across handlers
#[derive(Clone, Debug)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,
    /// Cache for expensive operations
    pub cache: Cache<String, CachedData>,
}

/// Cached data types
#[derive(Clone, Debug)]
pub enum CachedData {
    Statistics(AccountStatisticsCollection),
    Timeseries(AccountStateTimeseries),
}

/// Query parameters for statistics endpoints
#[derive(Debug, Deserialize, ToSchema)]
pub struct StatisticsQuery {
    /// Year for statistics (e.g., 2024)
    pub year: Option<i32>,
    /// Month for statistics (1-12)
    pub month: Option<u32>,
    /// Start date for custom range (YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// End date for custom range (YYYY-MM-DD)
    pub end_date: Option<NaiveDate>,
}

/// Query parameters for timeseries endpoints
#[derive(Debug, Deserialize, ToSchema)]
pub struct TimeseriesQuery {
    /// Start date for timeseries (YYYY-MM-DD)
    pub start_date: NaiveDate,
    /// End date for timeseries (YYYY-MM-DD)
    pub end_date: NaiveDate,
    /// Account IDs to include (comma-separated)
    pub account_ids: Option<String>,
}

/// API response wrapper
#[derive(Serialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Response message
    pub message: String,
    /// Success status
    pub success: bool,
}

/// Error response
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Error code
    pub code: String,
    /// Success status (always false for errors)
    pub success: bool,
}

/// Health check response
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
    /// Database connection status
    pub database: String,
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        get_account_statistics,
        get_account_timeseries,
        get_all_accounts_statistics,
        get_all_accounts_timeseries,
    ),
    components(
        schemas(
            ApiResponse<AccountStatisticsCollection>,
            ApiResponse<AccountStateTimeseries>,
            ApiResponse<Vec<AccountStatisticsCollection>>,
            ErrorResponse,
            HealthResponse,
            StatisticsQuery,
            TimeseriesQuery,
            AccountStatisticsCollection,
            AccountStatistics,
            TimePeriod,
            AccountStateTimeseries,
            DateRange,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "statistics", description = "Account statistics endpoints"),
        (name = "timeseries", description = "Account timeseries endpoints"),
    ),
    info(
        title = "FinRust API",
        description = "Home Finance Tracker API - A comprehensive financial tracking and analysis system",
        version = "0.1.0",
        contact(
            name = "FinRust Team",
            email = "contact@finrust.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    )
)]
struct ApiDoc;


/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 500, description = "Service is unhealthy", body = ErrorResponse)
    )
)]
#[instrument]
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
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

/// Get statistics for a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/statistics",
    tag = "statistics",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account statistics retrieved successfully", body = ApiResponse<AccountStatisticsCollection>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
async fn get_account_statistics(
    Path(account_id): Path<i32>,
    Query(query): Query<StatisticsQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStatisticsCollection>>, StatusCode> {
    // Create cache key
    let cache_key = format!("stats_{}_{:?}", account_id, query);

    // Check cache first
    if let Some(CachedData::Statistics(stats)) = state.cache.get(&cache_key).await {
        let response = ApiResponse {
            data: stats,
            message: "Account statistics retrieved from cache".to_string(),
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

    let period = determine_time_period(&query);

    // Compute actual statistics using compute module
    let accounts = vec![account_model];
    let compute = default_compute(None);

    // For now, use a simplified approach to get it compiling
    // We'll compute min and max state as an example of real compute module usage
    let statistics = match &period {
        TimePeriod::Year(year) => {
            let min_stats = account_stats::min_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year).await
                .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year).await
                .unwrap_or_else(|_| vec![]);

            // Compute additional statistics using compute module
            let avg_expense_stats = account_stats::average_expense_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year).await
                .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year).await
                .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, chrono::Utc::now().date_naive()).await
                .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_year_state(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year).await
                .unwrap_or_else(|_| vec![]);

            vec![AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
            }]
        },
        TimePeriod::Month { year, month } => {
            let min_stats = account_stats::min_state_in_month(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month).await
                .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_month(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month).await
                .unwrap_or_else(|_| vec![]);

            // Compute additional statistics using compute module
            let avg_expense_stats = account_stats::average_expense_in_month(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month).await
                .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_month(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month).await
                .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_month_end(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month, chrono::Utc::now().date_naive()).await
                .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_month_state(&compute as &dyn AccountStateCalculator, &state.db, &accounts, *year, *month).await
                .unwrap_or_else(|_| vec![]);

            vec![AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
            }]
        },
        TimePeriod::DateRange { start, end: _ } => {
            let year = start.year();
            let min_stats = account_stats::min_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year).await
                .unwrap_or_else(|_| vec![]);
            let max_stats = account_stats::max_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year).await
                .unwrap_or_else(|_| vec![]);

            // Compute additional statistics using compute module
            let avg_expense_stats = account_stats::average_expense_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year).await
                .unwrap_or_else(|_| vec![]);
            let avg_income_stats = account_stats::average_income_in_year(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year).await
                .unwrap_or_else(|_| vec![]);
            let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year, chrono::Utc::now().date_naive()).await
                .unwrap_or_else(|_| vec![]);
            let end_of_period_stats = account_stats::end_of_year_state(&compute as &dyn AccountStateCalculator, &state.db, &accounts, year).await
                .unwrap_or_else(|_| vec![]);

            vec![AccountStatistics {
                account_id,
                min_state: min_stats.first().and_then(|s| s.min_state),
                max_state: max_stats.first().and_then(|s| s.max_state),
                average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                average_income: avg_income_stats.first().and_then(|s| s.average_income),
                upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
            }]
        }
    };

    let collection = AccountStatisticsCollection::new(period, statistics);

    // Cache the result
    state.cache.insert(cache_key, CachedData::Statistics(collection.clone())).await;

    let response = ApiResponse {
        data: collection,
        message: "Account statistics retrieved successfully".to_string(),
        success: true,
    };

    Ok(Json(response))
}

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
async fn get_account_timeseries(
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

/// Get statistics for all accounts
#[utoipa::path(
    get,
    path = "/api/v1/accounts/statistics",
    tag = "statistics",
    responses(
        (status = 200, description = "All accounts statistics retrieved successfully", body = ApiResponse<Vec<AccountStatisticsCollection>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
async fn get_all_accounts_statistics(
    Query(query): Query<StatisticsQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AccountStatisticsCollection>>>, StatusCode> {
    // Get all accounts that are included in statistics
    let accounts = match account::Entity::find().all(&state.db).await {
        Ok(accounts) => accounts.into_iter().filter(|a| a.include_in_statistics).collect::<Vec<_>>(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if accounts.is_empty() {
        let response = ApiResponse {
            data: vec![],
            message: "No accounts found for statistics".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }

    // Implement actual statistics computation for all accounts using compute module
    let period = determine_time_period(&query);
    let mut all_statistics = Vec::new();
    let compute = default_compute(None);

    for account in accounts {
        let account_vec = vec![account.clone()];

        // Compute statistics for this account using the same logic as single account
        let statistics = match &period {
            TimePeriod::Year(year) => {
                let min_stats = account_stats::min_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year).await
                    .unwrap_or_else(|_| vec![]);
                let max_stats = account_stats::max_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year).await
                    .unwrap_or_else(|_| vec![]);
                let avg_expense_stats = account_stats::average_expense_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year).await
                    .unwrap_or_else(|_| vec![]);
                let avg_income_stats = account_stats::average_income_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year).await
                    .unwrap_or_else(|_| vec![]);
                let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, chrono::Utc::now().date_naive()).await
                    .unwrap_or_else(|_| vec![]);
                let end_of_period_stats = account_stats::end_of_year_state(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year).await
                    .unwrap_or_else(|_| vec![]);

                vec![AccountStatistics {
                    account_id: account.id,
                    min_state: min_stats.first().and_then(|s| s.min_state),
                    max_state: max_stats.first().and_then(|s| s.max_state),
                    average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                    average_income: avg_income_stats.first().and_then(|s| s.average_income),
                    upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                    end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
                }]
            },
            TimePeriod::Month { year, month } => {
                let min_stats = account_stats::min_state_in_month(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month).await
                    .unwrap_or_else(|_| vec![]);
                let max_stats = account_stats::max_state_in_month(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month).await
                    .unwrap_or_else(|_| vec![]);
                let avg_expense_stats = account_stats::average_expense_in_month(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month).await
                    .unwrap_or_else(|_| vec![]);
                let avg_income_stats = account_stats::average_income_in_month(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month).await
                    .unwrap_or_else(|_| vec![]);
                let upcoming_expenses_stats = account_stats::upcoming_expenses_until_month_end(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month, chrono::Utc::now().date_naive()).await
                    .unwrap_or_else(|_| vec![]);
                let end_of_period_stats = account_stats::end_of_month_state(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, *year, *month).await
                    .unwrap_or_else(|_| vec![]);

                vec![AccountStatistics {
                    account_id: account.id,
                    min_state: min_stats.first().and_then(|s| s.min_state),
                    max_state: max_stats.first().and_then(|s| s.max_state),
                    average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                    average_income: avg_income_stats.first().and_then(|s| s.average_income),
                    upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                    end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
                }]
            },
            TimePeriod::DateRange { start, end: _ } => {
                let year = start.year();
                let min_stats = account_stats::min_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year).await
                    .unwrap_or_else(|_| vec![]);
                let max_stats = account_stats::max_state_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year).await
                    .unwrap_or_else(|_| vec![]);
                let avg_expense_stats = account_stats::average_expense_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year).await
                    .unwrap_or_else(|_| vec![]);
                let avg_income_stats = account_stats::average_income_in_year(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year).await
                    .unwrap_or_else(|_| vec![]);
                let upcoming_expenses_stats = account_stats::upcoming_expenses_until_year_end(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year, chrono::Utc::now().date_naive()).await
                    .unwrap_or_else(|_| vec![]);
                let end_of_period_stats = account_stats::end_of_year_state(&compute as &dyn AccountStateCalculator, &state.db, &account_vec, year).await
                    .unwrap_or_else(|_| vec![]);

                vec![AccountStatistics {
                    account_id: account.id,
                    min_state: min_stats.first().and_then(|s| s.min_state),
                    max_state: max_stats.first().and_then(|s| s.max_state),
                    average_expense: avg_expense_stats.first().and_then(|s| s.average_expense),
                    average_income: avg_income_stats.first().and_then(|s| s.average_income),
                    upcoming_expenses: upcoming_expenses_stats.first().and_then(|s| s.upcoming_expenses),
                    end_of_period_state: end_of_period_stats.first().and_then(|s| s.end_of_period_state),
                }]
            }
        };

        let collection = AccountStatisticsCollection::new(period.clone(), statistics);
        all_statistics.push(collection);
    }

    let response = ApiResponse {
        data: all_statistics,
        message: "All accounts statistics retrieved successfully".to_string(),
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
async fn get_all_accounts_timeseries(
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

/// Helper function to determine time period from query parameters
fn determine_time_period(query: &StatisticsQuery) -> TimePeriod {
    if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        TimePeriod::date_range(start, end)
    } else if let (Some(year), Some(month)) = (query.year, query.month) {
        TimePeriod::month(year, month)
    } else if let Some(year) = query.year {
        TimePeriod::year(year)
    } else {
        // Default to current year
        TimePeriod::year(chrono::Utc::now().year())
    }
}

/// Helper function to convert DataFrame to AccountStateTimeseries
fn convert_dataframe_to_timeseries(df: polars::prelude::DataFrame) -> Result<AccountStateTimeseries, String> {
    use common::AccountStatePoint;
    use polars::prelude::*;

    // Extract columns from DataFrame
    let account_id_col = df.column("account_id").map_err(|e| format!("Missing account_id column: {}", e))?;
    let date_col = df.column("date").map_err(|e| format!("Missing date column: {}", e))?;
    let balance_col = df.column("balance").map_err(|e| format!("Missing balance column: {}", e))?;

    let mut data_points = Vec::new();

    // Iterate through rows and create AccountStatePoint objects
    for i in 0..df.height() {
        let account_id = account_id_col.get(i).map_err(|e| format!("Error getting account_id at row {}: {}", i, e))?
            .try_extract::<i32>().map_err(|e| format!("Error extracting account_id as i32 at row {}: {}", i, e))?;

        let date = date_col.get(i).map_err(|e| format!("Error getting date at row {}: {}", i, e))?
            .try_extract::<i64>().map_err(|e| format!("Error extracting date as i64 at row {}: {}", i, e))?;
        let naive_date = chrono::NaiveDate::from_num_days_from_ce_opt(date as i32)
            .ok_or_else(|| format!("Invalid date value at row {}: {}", i, date))?;

        let balance_str = match balance_col.get(i).map_err(|e| format!("Error getting balance at row {}: {}", i, e))? {
            polars::prelude::AnyValue::String(s) => s.to_string(),
            polars::prelude::AnyValue::StringOwned(s) => s.to_string(),
            other => format!("{}", other),
        };
        let balance = rust_decimal::Decimal::from_str(&balance_str)
            .map_err(|e| format!("Error parsing balance '{}' at row {}: {}", balance_str, i, e))?;

        data_points.push(AccountStatePoint::new(account_id, naive_date, balance));
    }

    Ok(AccountStateTimeseries::new(data_points))
}


/// Create application router with all routes and middleware
fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))

        // API v1 routes
        .route("/api/v1/accounts/:account_id/statistics", get(get_account_statistics))
        .route("/api/v1/accounts/:account_id/timeseries", get(get_account_timeseries))
        .route("/api/v1/accounts/statistics", get(get_all_accounts_statistics))
        .route("/api/v1/accounts/timeseries", get(get_all_accounts_timeseries))

        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))

        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

/// Main entry point for the FinRust application.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finrust=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("FinRust application starting up");

    // Load configuration
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://finrust.db".to_string());
    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    // Connect to database
    info!("Connecting to database: {}", database_url);
    let db = Database::connect(&database_url).await?;

    // Initialize cache
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300)) // 5 minutes
        .build();

    // Create application state
    let state = AppState { db, cache };

    // Create router
    let app = create_router(state);

    // Start server
    info!("Starting server on {}", bind_address);
    let listener = TcpListener::bind(&bind_address).await?;

    info!("FinRust API server running on http://{}", bind_address);
    info!("Swagger UI available at http://{}/swagger-ui", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
