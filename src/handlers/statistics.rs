use axum::{extract::{Path, Query, State}, http::StatusCode, response::Json};
use common::AccountStatisticsCollection;
use model::entities::account;
use sea_orm::EntityTrait;
use tracing::instrument;
use crate::schemas::{AppState, ApiResponse, StatisticsQuery, CachedData};
use crate::helpers::stats::{determine_time_period, compute_account_statistics};

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
        (status = 404, description = "Account not found", body = crate::schemas::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::schemas::ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_statistics(
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

    // Compute statistics using helper function
    let statistics = match compute_account_statistics(&state.db, &account_model, &period).await {
        Ok(stats) => vec![stats],
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
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

/// Get statistics for all accounts
#[utoipa::path(
    get,
    path = "/api/v1/accounts/statistics",
    tag = "statistics",
    responses(
        (status = 200, description = "All accounts statistics retrieved successfully", body = ApiResponse<Vec<AccountStatisticsCollection>>),
        (status = 500, description = "Internal server error", body = crate::schemas::ErrorResponse)
    )
)]
#[instrument]
pub async fn get_all_accounts_statistics(
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

    let period = determine_time_period(&query);
    let mut all_statistics = Vec::new();

    for account in accounts {
        // Compute statistics for this account using helper function
        let statistics = match compute_account_statistics(&state.db, &account, &period).await {
            Ok(stats) => vec![stats],
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
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
