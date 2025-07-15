use chrono::NaiveDate;
use common::{
    AccountStateTimeseries, AccountStatistics, AccountStatisticsCollection, DateRange, TimePeriod,
};
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use validator::Validate;

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
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct StatisticsQuery {
    /// Year for statistics (e.g., 2024)
    pub year: Option<i32>,
    /// Month for statistics (1-12)
    #[validate(range(min = 1, max = 12))]
    pub month: Option<u32>,
    /// Start date for custom range (YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// End date for custom range (YYYY-MM-DD)
    pub end_date: Option<NaiveDate>,
}

/// Query parameters for timeseries endpoints
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[validate(schema(function = "validate_timeseries_dates"))]
pub struct TimeseriesQuery {
    /// Start date for timeseries (YYYY-MM-DD)
    pub start_date: NaiveDate,
    /// End date for timeseries (YYYY-MM-DD)
    pub end_date: NaiveDate,
}

fn validate_timeseries_dates(query: &TimeseriesQuery) -> Result<(), validator::ValidationError> {
    if query.start_date >= query.end_date {
        return Err(validator::ValidationError::new("start_date must be before end_date"));
    }
    Ok(())
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
        crate::handlers::manual_account_states::create_manual_account_state,
        crate::handlers::manual_account_states::get_manual_account_states,
        crate::handlers::manual_account_states::get_manual_account_state,
        crate::handlers::manual_account_states::update_manual_account_state,
        crate::handlers::manual_account_states::delete_manual_account_state,
        crate::handlers::users::create_user,
        crate::handlers::users::get_users,
        crate::handlers::users::get_user,
        crate::handlers::users::update_user,
        crate::handlers::users::delete_user,
        crate::handlers::transactions::create_transaction,
        crate::handlers::transactions::get_transactions,
        crate::handlers::transactions::get_account_transactions,
        crate::handlers::transactions::get_transaction,
        crate::handlers::transactions::update_transaction,
        crate::handlers::transactions::delete_transaction,
        crate::handlers::transactions::create_recurring_transaction,
        crate::handlers::transactions::get_recurring_transactions,
        crate::handlers::transactions::get_recurring_transaction,
        crate::handlers::transactions::update_recurring_transaction,
        crate::handlers::transactions::delete_recurring_transaction,
        crate::handlers::transactions::create_recurring_instance,
        crate::handlers::transactions::create_imported_transaction,
        crate::handlers::transactions::get_imported_transactions,
        crate::handlers::transactions::get_account_imported_transactions,
        crate::handlers::transactions::get_imported_transaction,
        crate::handlers::transactions::update_imported_transaction,
        crate::handlers::transactions::delete_imported_transaction,
        crate::handlers::transactions::reconcile_imported_transaction,
        crate::handlers::transactions::clear_imported_transaction_reconciliation,
        crate::handlers::recurring_income::create_recurring_income,
        crate::handlers::recurring_income::get_recurring_incomes,
        crate::handlers::recurring_income::get_recurring_income,
        crate::handlers::recurring_income::update_recurring_income,
        crate::handlers::recurring_income::delete_recurring_income,
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
            ApiResponse<crate::handlers::users::UserResponse>,
            ApiResponse<Vec<crate::handlers::users::UserResponse>>,
            ApiResponse<crate::handlers::transactions::TransactionResponse>,
            ApiResponse<Vec<crate::handlers::transactions::TransactionResponse>>,
            ApiResponse<String>,
            crate::handlers::accounts::CreateAccountRequest,
            crate::handlers::accounts::UpdateAccountRequest,
            crate::handlers::accounts::AccountResponse,
            crate::handlers::manual_account_states::CreateManualAccountStateRequest,
            crate::handlers::manual_account_states::UpdateManualAccountStateRequest,
            crate::handlers::manual_account_states::ManualAccountStateResponse,
            ApiResponse<crate::handlers::manual_account_states::ManualAccountStateResponse>,
            ApiResponse<Vec<crate::handlers::manual_account_states::ManualAccountStateResponse>>,
            crate::handlers::users::CreateUserRequest,
            crate::handlers::users::UpdateUserRequest,
            crate::handlers::users::UserResponse,
            crate::handlers::transactions::CreateTransactionRequest,
            crate::handlers::transactions::UpdateTransactionRequest,
            crate::handlers::transactions::TransactionResponse,
            crate::handlers::transactions::CreateRecurringTransactionRequest,
            crate::handlers::transactions::UpdateRecurringTransactionRequest,
            crate::handlers::transactions::RecurringTransactionResponse,
            crate::handlers::transactions::RecurringTransactionQuery,
            crate::handlers::transactions::CreateRecurringInstanceRequest,
            crate::handlers::transactions::RecurringInstanceResponse,
            crate::handlers::transactions::CreateImportedTransactionRequest,
            crate::handlers::transactions::UpdateImportedTransactionRequest,
            crate::handlers::transactions::ImportedTransactionResponse,
            crate::handlers::transactions::ReconcileImportedTransactionRequest,
            crate::handlers::transactions::ReconciledTransactionInfo,
            crate::handlers::transactions::ImportedTransactionQuery,
            ApiResponse<crate::handlers::transactions::ImportedTransactionResponse>,
            ApiResponse<Vec<crate::handlers::transactions::ImportedTransactionResponse>>,
            crate::handlers::recurring_income::CreateRecurringIncomeRequest,
            crate::handlers::recurring_income::UpdateRecurringIncomeRequest,
            crate::handlers::recurring_income::RecurringIncomeResponse,
            crate::handlers::recurring_income::RecurringIncomeQuery,
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
        (name = "manual-account-states", description = "Manual account state CRUD operations"),
        (name = "users", description = "User CRUD operations"),
        (name = "transactions", description = "Transaction CRUD operations"),
        (name = "recurring-transactions", description = "Recurring transaction operations"),
        (name = "imported-transactions", description = "Imported transaction CRUD operations and reconciliation"),
        (name = "recurring-incomes", description = "Recurring income operations"),
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
