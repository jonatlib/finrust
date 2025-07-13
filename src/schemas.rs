use chrono::NaiveDate;
use common::{
    AccountStateTimeseries, AccountStatistics, AccountStatisticsCollection, DateRange, TimePeriod,
};
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

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
}

/// API response wrapper
#[derive(Serialize, Deserialize, ToSchema)]
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
        crate::handlers::health::health_check,
        crate::handlers::accounts::create_account,
        crate::handlers::accounts::get_accounts,
        crate::handlers::accounts::get_account,
        crate::handlers::accounts::update_account,
        crate::handlers::accounts::delete_account,
        crate::handlers::transactions::create_transaction,
        crate::handlers::transactions::get_transactions,
        crate::handlers::transactions::get_account_transactions,
        crate::handlers::transactions::get_transaction,
        crate::handlers::transactions::update_transaction,
        crate::handlers::transactions::delete_transaction,
        crate::handlers::statistics::get_account_statistics,
        crate::handlers::timeseries::get_account_timeseries,
        crate::handlers::statistics::get_all_accounts_statistics,
        crate::handlers::timeseries::get_all_accounts_timeseries,
    ),
    components(
        schemas(
            ApiResponse<AccountStatisticsCollection>,
            ApiResponse<AccountStateTimeseries>,
            ApiResponse<Vec<AccountStatisticsCollection>>,
            ApiResponse<crate::handlers::accounts::AccountResponse>,
            ApiResponse<Vec<crate::handlers::accounts::AccountResponse>>,
            ApiResponse<crate::handlers::transactions::TransactionResponse>,
            ApiResponse<Vec<crate::handlers::transactions::TransactionResponse>>,
            ApiResponse<String>,
            crate::handlers::accounts::CreateAccountRequest,
            crate::handlers::accounts::UpdateAccountRequest,
            crate::handlers::accounts::AccountResponse,
            crate::handlers::transactions::CreateTransactionRequest,
            crate::handlers::transactions::UpdateTransactionRequest,
            crate::handlers::transactions::TransactionResponse,
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
        (name = "accounts", description = "Account CRUD operations"),
        (name = "transactions", description = "Transaction CRUD operations"),
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
pub struct ApiDoc;
