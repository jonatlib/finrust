use crate::helpers::stats::{compute_account_statistics, determine_time_period};
use crate::schemas::{ApiResponse, AppState, CachedData, MonthlyMinBalanceQuery, StatisticsQuery, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use chrono::{Datelike, Months};
use common::{AccountStatisticsCollection, MonthlyMinBalance, MonthlyMinBalanceSeries};
use compute::{account::AccountStateCalculator, account_stats, default_compute};
use model::entities::account;
use sea_orm::EntityTrait;
use tracing::{instrument, error, warn, info, debug, trace};

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
pub async fn get_account_statistics(
    Path(account_id): Path<i32>,
    Valid(Query(query)): Valid<Query<StatisticsQuery>>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountStatisticsCollection>>, StatusCode> {
    trace!("Entering get_account_statistics function for account_id: {}", account_id);
    debug!("Fetching statistics for account ID: {} with query: {:?}", account_id, query);

    // Create cache key
    let cache_key = format!("stats_{}_{:?}", account_id, query);
    trace!("Generated cache key: {}", cache_key);

    // Check cache first
    debug!("Checking cache for statistics");
    if let Some(CachedData::Statistics(stats)) = state.cache.get(&cache_key).await {
        info!("Statistics for account ID {} retrieved from cache", account_id);
        let response = ApiResponse {
            data: stats,
            message: "Account statistics retrieved from cache".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }
    debug!("Cache miss for account statistics, proceeding with database query");

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

    let period = determine_time_period(&query);
    debug!("Determined time period: {:?}", period);

    // Compute statistics using helper function
    trace!("Computing statistics for account: {}", account_model.name);
    let statistics = match compute_account_statistics(&state.db, &account_model, &period).await {
        Ok(stats) => {
            debug!("Successfully computed statistics for account ID: {}", account_id);
            vec![stats]
        }
        Err(compute_error) => {
            error!("Failed to compute statistics for account ID {}: {}", account_id, compute_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let collection = AccountStatisticsCollection::new(period, statistics);

    // Cache the result
    trace!("Caching statistics result with key: {}", cache_key);
    state
        .cache
        .insert(cache_key, CachedData::Statistics(collection.clone()))
        .await;
    debug!("Statistics cached successfully");

    info!("Account statistics for ID {} retrieved and cached successfully", account_id);
    let response = ApiResponse {
        data: collection,
        message: "Account statistics retrieved successfully".to_string(),
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
pub async fn get_all_accounts_statistics(
    Valid(Query(query)): Valid<Query<StatisticsQuery>>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AccountStatisticsCollection>>>, StatusCode> {
    trace!("Entering get_all_accounts_statistics function");
    debug!("Fetching statistics for all accounts with query: {:?}", query);

    // Get all accounts that are included in statistics
    trace!("Querying database for all accounts");
    let accounts = match account::Entity::find().all(&state.db).await {
        Ok(accounts) => {
            let all_count = accounts.len();
            let filtered_accounts: Vec<_> = accounts
                .into_iter()
                .filter(|a| query.include_ignored || a.include_in_statistics)
                .collect();
            let filtered_count = filtered_accounts.len();
            debug!("Retrieved {} total accounts, {} after filtering (include_ignored={})", all_count, filtered_count, query.include_ignored);
            filtered_accounts
        }
        Err(db_error) => {
            error!("Failed to retrieve accounts from database: {}", db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if accounts.is_empty() {
        warn!("No accounts found that are included in statistics");
        let response = ApiResponse {
            data: vec![],
            message: "No accounts found for statistics".to_string(),
            success: true,
        };
        return Ok(Json(response));
    }

    let period = determine_time_period(&query);
    debug!("Determined time period: {:?}", period);
    let mut all_statistics = Vec::new();

    for account in accounts {
        trace!("Computing statistics for account: {} (ID: {})", account.name, account.id);
        // Compute statistics for this account using helper function
        let statistics = match compute_account_statistics(&state.db, &account, &period).await {
            Ok(stats) => {
                debug!("Successfully computed statistics for account: {}", account.name);
                vec![stats]
            }
            Err(compute_error) => {
                error!("Failed to compute statistics for account {} (ID: {}): {}", 
                       account.name, account.id, compute_error);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let collection = AccountStatisticsCollection::new(period.clone(), statistics);
        all_statistics.push(collection);
    }

    info!("Successfully computed statistics for {} accounts", all_statistics.len());
    let response = ApiResponse {
        data: all_statistics,
        message: "All accounts statistics retrieved successfully".to_string(),
        success: true,
    };

    Ok(Json(response))
}

/// Get monthly minimum balance for a specific account over time.
///
/// Returns the minimum balance observed during each calendar month,
/// allowing the user to see whether the account floor is trending
/// upward or downward.
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/monthly-min-balance",
    tag = "statistics",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
        ("months" = Option<u32>, Query, description = "Number of past months to include (default 12)"),
    ),
    responses(
        (status = 200, description = "Monthly minimum balance series", body = ApiResponse<MonthlyMinBalanceSeries>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_monthly_min_balance(
    Path(account_id): Path<i32>,
    Valid(Query(query)): Valid<Query<MonthlyMinBalanceQuery>>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MonthlyMinBalanceSeries>>, StatusCode> {
    trace!("Entering get_monthly_min_balance for account_id: {}", account_id);

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

    let months = query.months.unwrap_or(12);
    let today = chrono::Utc::now().date_naive();
    let end_date = today;
    let start_date = today
        .checked_sub_months(Months::new(months))
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(today.year() - 1, today.month(), 1).unwrap());

    let compute = default_compute(None);

    debug!("Computing monthly min balance from {} to {} ({} months)", start_date, end_date, months);

    match account_stats::min_balance_per_month(
        &compute as &dyn AccountStateCalculator,
        &state.db,
        &account_model,
        start_date,
        end_date,
    )
    .await
    {
        Ok(raw_points) => {
            let data_points: Vec<MonthlyMinBalance> = raw_points
                .into_iter()
                .map(|(year, _date, min_bal)| MonthlyMinBalance {
                    year,
                    month: _date.month(),
                    min_balance: min_bal,
                })
                .collect();

            let series = MonthlyMinBalanceSeries {
                account_id,
                data_points,
            };

            info!(
                account_id,
                point_count = series.data_points.len(),
                "Monthly min balance computed successfully"
            );

            Ok(Json(ApiResponse {
                data: series,
                message: "Monthly minimum balance retrieved successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!(
                "Failed to compute monthly min balance for account ID {}: {}",
                account_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
