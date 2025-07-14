use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use chrono::NaiveDate;
use model::entities::{recurring_income, recurring_transaction};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, PaginatorTrait, QueryOrder, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::{ToSchema, IntoParams};
use validator::Validate;

/// Request body for creating a recurring income
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateRecurringIncomeRequest {
    /// Name of the recurring income
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Amount (expected to be positive for income)
    pub amount: Decimal,
    /// Start date for the recurring income
    pub start_date: NaiveDate,
    /// Optional end date (if not provided, repeats indefinitely)
    pub end_date: Option<NaiveDate>,
    /// Recurrence period
    pub period: String, // Will be parsed to RecurrencePeriod
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Target account ID where income is deposited
    pub target_account_id: i32,
    /// Optional source name (e.g., "Company XYZ")
    pub source_name: Option<String>,
    /// Optional ledger name
    pub ledger_name: Option<String>,
}

/// Request body for updating a recurring income
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateRecurringIncomeRequest {
    /// Name of the recurring income
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Amount (expected to be positive for income)
    pub amount: Option<Decimal>,
    /// Start date for the recurring income
    pub start_date: Option<NaiveDate>,
    /// Optional end date (if not provided, repeats indefinitely)
    pub end_date: Option<NaiveDate>,
    /// Recurrence period
    pub period: Option<String>, // Will be parsed to RecurrencePeriod
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Target account ID where income is deposited
    pub target_account_id: Option<i32>,
    /// Optional source name (e.g., "Company XYZ")
    pub source_name: Option<String>,
    /// Optional ledger name
    pub ledger_name: Option<String>,
}

/// Recurring income response model
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecurringIncomeResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub period: String,
    pub include_in_statistics: bool,
    pub target_account_id: i32,
    pub source_name: Option<String>,
    pub ledger_name: Option<String>,
}

impl From<recurring_income::Model> for RecurringIncomeResponse {
    fn from(model: recurring_income::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            amount: model.amount,
            start_date: model.start_date,
            end_date: model.end_date,
            period: format!("{:?}", model.period),
            include_in_statistics: model.include_in_statistics,
            target_account_id: model.target_account_id,
            source_name: model.source_name,
            ledger_name: model.ledger_name,
        }
    }
}

/// Query parameters for listing recurring incomes
#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct RecurringIncomeQuery {
    /// Page number (default: 1)
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    /// Page size (default: 50)
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<u64>,
    /// Filter by target account ID
    pub target_account_id: Option<i32>,
}

// Helper function to parse period string to RecurrencePeriod enum
fn parse_recurrence_period(period_str: &str) -> Result<recurring_transaction::RecurrencePeriod, String> {
    match period_str {
        "Daily" => Ok(recurring_transaction::RecurrencePeriod::Daily),
        "Weekly" => Ok(recurring_transaction::RecurrencePeriod::Weekly),
        "WorkDay" => Ok(recurring_transaction::RecurrencePeriod::WorkDay),
        "Monthly" => Ok(recurring_transaction::RecurrencePeriod::Monthly),
        "Quarterly" => Ok(recurring_transaction::RecurrencePeriod::Quarterly),
        "HalfYearly" => Ok(recurring_transaction::RecurrencePeriod::HalfYearly),
        "Yearly" => Ok(recurring_transaction::RecurrencePeriod::Yearly),
        _ => Err(format!("Invalid recurrence period: {}", period_str)),
    }
}

/// Create a new recurring income
#[utoipa::path(
    post,
    path = "/api/v1/recurring-incomes",
    tag = "recurring-incomes",
    request_body = CreateRecurringIncomeRequest,
    responses(
        (status = 201, description = "Recurring income created successfully", body = ApiResponse<RecurringIncomeResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_recurring_income(
    State(state): State<AppState>,
    Json(request): Json<CreateRecurringIncomeRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringIncomeResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_recurring_income function");
    debug!("Creating recurring income: {}", request.name);

    // Parse the recurrence period
    let period = match parse_recurrence_period(&request.period) {
        Ok(p) => p,
        Err(e) => {
            warn!("Invalid recurrence period: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e,
                    code: "INVALID_RECURRENCE_PERIOD".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Create the new recurring income
    let new_income = recurring_income::ActiveModel {
        name: Set(request.name),
        description: Set(request.description),
        amount: Set(request.amount),
        start_date: Set(request.start_date),
        end_date: Set(request.end_date),
        period: Set(period),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        target_account_id: Set(request.target_account_id),
        source_name: Set(request.source_name),
        ledger_name: Set(request.ledger_name),
        ..Default::default()
    };

    match new_income.insert(&state.db).await {
        Ok(income) => {
            info!("Successfully created recurring income with ID: {}", income.id);
            let response = ApiResponse {
                data: RecurringIncomeResponse::from(income),
                message: "Recurring income created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            error!("Failed to create recurring income: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create recurring income".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Get all recurring incomes
#[utoipa::path(
    get,
    path = "/api/v1/recurring-incomes",
    tag = "recurring-incomes",
    params(RecurringIncomeQuery),
    responses(
        (status = 200, description = "Recurring incomes retrieved successfully", body = ApiResponse<Vec<RecurringIncomeResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_incomes(
    Valid(Query(query)): Valid<Query<RecurringIncomeQuery>>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<RecurringIncomeResponse>>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_incomes function");

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50);

    debug!("Fetching recurring incomes - page: {}, limit: {}", page, limit);

    let mut query_builder = recurring_income::Entity::find();

    // Apply filters
    if let Some(target_account_id) = query.target_account_id {
        query_builder = query_builder.filter(recurring_income::Column::TargetAccountId.eq(target_account_id));
    }

    match query_builder
        .order_by_asc(recurring_income::Column::Id)
        .paginate(&state.db, limit)
        .fetch_page(page - 1)
        .await
    {
        Ok(incomes) => {
            info!("Successfully retrieved {} recurring incomes", incomes.len());
            let response_data: Vec<RecurringIncomeResponse> = incomes
                .into_iter()
                .map(RecurringIncomeResponse::from)
                .collect();

            let response = ApiResponse {
                data: response_data,
                message: "Recurring incomes retrieved successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            error!("Failed to retrieve recurring incomes: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring incomes".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Get a specific recurring income by ID
#[utoipa::path(
    get,
    path = "/api/v1/recurring-incomes/{recurring_income_id}",
    tag = "recurring-incomes",
    responses(
        (status = 200, description = "Recurring income retrieved successfully", body = ApiResponse<RecurringIncomeResponse>),
        (status = 404, description = "Recurring income not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_income(
    Path(recurring_income_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringIncomeResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_income function");
    debug!("Fetching recurring income with ID: {}", recurring_income_id);

    match recurring_income::Entity::find_by_id(recurring_income_id)
        .one(&state.db)
        .await
    {
        Ok(Some(income)) => {
            info!("Successfully retrieved recurring income: {}", income.name);
            let response = ApiResponse {
                data: RecurringIncomeResponse::from(income),
                message: "Recurring income retrieved successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Ok(None) => {
            warn!("Recurring income with ID {} not found", recurring_income_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring income with id {} does not exist", recurring_income_id),
                    code: "RECURRING_INCOME_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while fetching recurring income: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring income".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Update a recurring income
#[utoipa::path(
    put,
    path = "/api/v1/recurring-incomes/{recurring_income_id}",
    tag = "recurring-incomes",
    request_body = UpdateRecurringIncomeRequest,
    responses(
        (status = 200, description = "Recurring income updated successfully", body = ApiResponse<RecurringIncomeResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Recurring income not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_recurring_income(
    Path(recurring_income_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateRecurringIncomeRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringIncomeResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering update_recurring_income function");
    debug!("Updating recurring income with ID: {}", recurring_income_id);

    // First, fetch the existing income
    let existing_income = match recurring_income::Entity::find_by_id(recurring_income_id)
        .one(&state.db)
        .await
    {
        Ok(Some(income)) => income,
        Ok(None) => {
            warn!("Recurring income with ID {} not found", recurring_income_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring income with id {} does not exist", recurring_income_id),
                    code: "RECURRING_INCOME_NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Database error while fetching recurring income: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring income".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Parse period if provided
    let period = if let Some(period_str) = &request.period {
        match parse_recurrence_period(period_str) {
            Ok(p) => Some(p),
            Err(e) => {
                warn!("Invalid recurrence period: {}", e);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e,
                        code: "INVALID_RECURRENCE_PERIOD".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    } else {
        None
    };

    // Create the update model
    let mut update_model: recurring_income::ActiveModel = existing_income.into();

    if let Some(name) = request.name {
        update_model.name = Set(name);
    }
    if let Some(description) = request.description {
        update_model.description = Set(Some(description));
    }
    if let Some(amount) = request.amount {
        update_model.amount = Set(amount);
    }
    if let Some(start_date) = request.start_date {
        update_model.start_date = Set(start_date);
    }
    if let Some(end_date) = request.end_date {
        update_model.end_date = Set(Some(end_date));
    }
    if let Some(p) = period {
        update_model.period = Set(p);
    }
    if let Some(include_in_statistics) = request.include_in_statistics {
        update_model.include_in_statistics = Set(include_in_statistics);
    }
    if let Some(target_account_id) = request.target_account_id {
        update_model.target_account_id = Set(target_account_id);
    }
    if let Some(source_name) = request.source_name {
        update_model.source_name = Set(Some(source_name));
    }
    if let Some(ledger_name) = request.ledger_name {
        update_model.ledger_name = Set(Some(ledger_name));
    }

    match update_model.update(&state.db).await {
        Ok(updated_income) => {
            info!("Successfully updated recurring income with ID: {}", updated_income.id);
            let response = ApiResponse {
                data: RecurringIncomeResponse::from(updated_income),
                message: "Recurring income updated successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            error!("Failed to update recurring income: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update recurring income".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Delete a recurring income
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-incomes/{recurring_income_id}",
    tag = "recurring-incomes",
    responses(
        (status = 200, description = "Recurring income deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Recurring income not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_recurring_income(
    Path(recurring_income_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering delete_recurring_income function");
    debug!("Deleting recurring income with ID: {}", recurring_income_id);

    // First, check if the income exists
    match recurring_income::Entity::find_by_id(recurring_income_id)
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            // Income exists, proceed with deletion
            match recurring_income::Entity::delete_by_id(recurring_income_id)
                .exec(&state.db)
                .await
            {
                Ok(_) => {
                    info!("Successfully deleted recurring income with ID: {}", recurring_income_id);
                    let response = ApiResponse {
                        data: format!("Recurring income with id {} deleted successfully", recurring_income_id),
                        message: "Recurring income deleted successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(e) => {
                    error!("Failed to delete recurring income: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to delete recurring income".to_string(),
                            code: "DATABASE_ERROR".to_string(),
                            success: false,
                        }),
                    ))
                }
            }
        }
        Ok(None) => {
            warn!("Recurring income with ID {} not found", recurring_income_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring income with id {} does not exist", recurring_income_id),
                    code: "RECURRING_INCOME_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while checking recurring income existence: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check recurring income existence".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}
