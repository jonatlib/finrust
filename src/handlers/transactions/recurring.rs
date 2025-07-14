use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::{recurring_transaction, recurring_transaction_instance};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, PaginatorTrait, QueryOrder, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::{ToSchema, IntoParams};

/// Request body for creating a recurring transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateRecurringTransactionRequest {
    /// Name of the recurring transaction
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Amount (positive for income, negative for expense)
    pub amount: Decimal,
    /// Start date for the recurring transaction
    pub start_date: NaiveDate,
    /// Optional end date (if not provided, repeats indefinitely)
    pub end_date: Option<NaiveDate>,
    /// Recurrence period
    pub period: String, // Will be parsed to RecurrencePeriod
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Target account ID
    pub target_account_id: i32,
    /// Optional source account ID for transfers
    pub source_account_id: Option<i32>,
    /// Optional ledger name
    pub ledger_name: Option<String>,
}

/// Request body for updating a recurring transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateRecurringTransactionRequest {
    /// Name of the recurring transaction
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Amount (positive for income, negative for expense)
    pub amount: Option<Decimal>,
    /// Start date for the recurring transaction
    pub start_date: Option<NaiveDate>,
    /// Optional end date (if not provided, repeats indefinitely)
    pub end_date: Option<NaiveDate>,
    /// Recurrence period
    pub period: Option<String>, // Will be parsed to RecurrencePeriod
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Target account ID
    pub target_account_id: Option<i32>,
    /// Optional source account ID for transfers
    pub source_account_id: Option<i32>,
    /// Optional ledger name
    pub ledger_name: Option<String>,
}

/// Recurring transaction response model
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecurringTransactionResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub period: String,
    pub include_in_statistics: bool,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
}

impl From<recurring_transaction::Model> for RecurringTransactionResponse {
    fn from(model: recurring_transaction::Model) -> Self {
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
            source_account_id: model.source_account_id,
            ledger_name: model.ledger_name,
        }
    }
}

/// Query parameters for listing recurring transactions
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct RecurringTransactionQuery {
    /// Page number (default: 1)
    pub page: Option<u64>,
    /// Page size (default: 50)
    pub limit: Option<u64>,
    /// Filter by target account ID
    pub target_account_id: Option<i32>,
    /// Filter by source account ID
    pub source_account_id: Option<i32>,
}

/// Request body for creating a recurring transaction instance
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateRecurringInstanceRequest {
    /// The date for this instance
    pub date: NaiveDate,
    /// Optional amount override (if not provided, uses original amount from recurring transaction)
    pub amount: Option<Decimal>,
}

/// Recurring transaction instance response model
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecurringInstanceResponse {
    pub id: i32,
    pub recurring_transaction_id: i32,
    pub status: String,
    pub due_date: NaiveDate,
    pub expected_amount: Decimal,
    pub paid_date: Option<NaiveDate>,
    pub paid_amount: Option<Decimal>,
    pub reconciled_imported_transaction_id: Option<i32>,
}

impl From<recurring_transaction_instance::Model> for RecurringInstanceResponse {
    fn from(model: recurring_transaction_instance::Model) -> Self {
        Self {
            id: model.id,
            recurring_transaction_id: model.recurring_transaction_id,
            status: format!("{:?}", model.status),
            due_date: model.due_date,
            expected_amount: model.expected_amount,
            paid_date: model.paid_date,
            paid_amount: model.paid_amount,
            reconciled_imported_transaction_id: model.reconciled_imported_transaction_id,
        }
    }
}

/// Create a new recurring transaction instance
#[utoipa::path(
    post,
    path = "/api/v1/recurring-transactions/{recurring_transaction_id}/instances",
    tag = "recurring-transactions",
    request_body = CreateRecurringInstanceRequest,
    responses(
        (status = 201, description = "Recurring transaction instance created successfully", body = ApiResponse<RecurringInstanceResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Recurring transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_recurring_instance(
    Path(recurring_transaction_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<CreateRecurringInstanceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringInstanceResponse>>), StatusCode> {
    trace!("Entering create_recurring_instance function");
    debug!("Creating instance for recurring transaction ID: {}, date: {}", 
           recurring_transaction_id, request.date);

    // First, fetch the recurring transaction to get the original amount
    let recurring_transaction = match recurring_transaction::Entity::find_by_id(recurring_transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction)) => {
            debug!("Found recurring transaction: {}", transaction.name);
            transaction
        }
        Ok(None) => {
            warn!("Recurring transaction with ID {} not found", recurring_transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Database error while fetching recurring transaction: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Use the provided amount or fall back to the original amount
    let instance_amount = request.amount.unwrap_or(recurring_transaction.amount);

    // Create the new recurring transaction instance
    let new_instance = recurring_transaction_instance::ActiveModel {
        recurring_transaction_id: Set(recurring_transaction_id),
        status: Set(recurring_transaction_instance::InstanceStatus::Pending),
        due_date: Set(request.date),
        expected_amount: Set(instance_amount),
        paid_date: Set(None),
        paid_amount: Set(None),
        reconciled_imported_transaction_id: Set(None),
        ..Default::default()
    };

    match new_instance.insert(&state.db).await {
        Ok(instance) => {
            info!("Successfully created recurring transaction instance with ID: {}", instance.id);
            let response = ApiResponse {
                data: RecurringInstanceResponse::from(instance),
                message: "Recurring transaction instance created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            error!("Failed to create recurring transaction instance: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

/// Create a new recurring transaction
#[utoipa::path(
    post,
    path = "/api/v1/recurring-transactions",
    tag = "recurring-transactions",
    request_body = CreateRecurringTransactionRequest,
    responses(
        (status = 201, description = "Recurring transaction created successfully", body = ApiResponse<RecurringTransactionResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_recurring_transaction(
    State(state): State<AppState>,
    Json(request): Json<CreateRecurringTransactionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringTransactionResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_recurring_transaction function");
    debug!("Creating recurring transaction: {}", request.name);

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

    // Create the new recurring transaction
    let new_transaction = recurring_transaction::ActiveModel {
        name: Set(request.name),
        description: Set(request.description),
        amount: Set(request.amount),
        start_date: Set(request.start_date),
        end_date: Set(request.end_date),
        period: Set(period),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        target_account_id: Set(request.target_account_id),
        source_account_id: Set(request.source_account_id),
        ledger_name: Set(request.ledger_name),
        ..Default::default()
    };

    match new_transaction.insert(&state.db).await {
        Ok(transaction) => {
            info!("Successfully created recurring transaction with ID: {}", transaction.id);
            let response = ApiResponse {
                data: RecurringTransactionResponse::from(transaction),
                message: "Recurring transaction created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            error!("Failed to create recurring transaction: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create recurring transaction".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Get all recurring transactions
#[utoipa::path(
    get,
    path = "/api/v1/recurring-transactions",
    tag = "recurring-transactions",
    params(RecurringTransactionQuery),
    responses(
        (status = 200, description = "Recurring transactions retrieved successfully", body = ApiResponse<Vec<RecurringTransactionResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_transactions(
    Query(query): Query<RecurringTransactionQuery>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<RecurringTransactionResponse>>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_transactions function");

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50);

    debug!("Fetching recurring transactions - page: {}, limit: {}", page, limit);

    let mut query_builder = recurring_transaction::Entity::find();

    // Apply filters
    if let Some(target_account_id) = query.target_account_id {
        query_builder = query_builder.filter(recurring_transaction::Column::TargetAccountId.eq(target_account_id));
    }

    if let Some(source_account_id) = query.source_account_id {
        query_builder = query_builder.filter(recurring_transaction::Column::SourceAccountId.eq(source_account_id));
    }

    match query_builder
        .order_by_asc(recurring_transaction::Column::Id)
        .paginate(&state.db, limit)
        .fetch_page(page - 1)
        .await
    {
        Ok(transactions) => {
            info!("Successfully retrieved {} recurring transactions", transactions.len());
            let response_data: Vec<RecurringTransactionResponse> = transactions
                .into_iter()
                .map(RecurringTransactionResponse::from)
                .collect();

            let response = ApiResponse {
                data: response_data,
                message: "Recurring transactions retrieved successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            error!("Failed to retrieve recurring transactions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring transactions".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Get a specific recurring transaction by ID
#[utoipa::path(
    get,
    path = "/api/v1/recurring-transactions/{recurring_transaction_id}",
    tag = "recurring-transactions",
    responses(
        (status = 200, description = "Recurring transaction retrieved successfully", body = ApiResponse<RecurringTransactionResponse>),
        (status = 404, description = "Recurring transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_transaction(
    Path(recurring_transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringTransactionResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_transaction function");
    debug!("Fetching recurring transaction with ID: {}", recurring_transaction_id);

    match recurring_transaction::Entity::find_by_id(recurring_transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction)) => {
            info!("Successfully retrieved recurring transaction: {}", transaction.name);
            let response = ApiResponse {
                data: RecurringTransactionResponse::from(transaction),
                message: "Recurring transaction retrieved successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Ok(None) => {
            warn!("Recurring transaction with ID {} not found", recurring_transaction_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring transaction with id {} does not exist", recurring_transaction_id),
                    code: "RECURRING_TRANSACTION_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while fetching recurring transaction: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring transaction".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Update a recurring transaction
#[utoipa::path(
    put,
    path = "/api/v1/recurring-transactions/{recurring_transaction_id}",
    tag = "recurring-transactions",
    request_body = UpdateRecurringTransactionRequest,
    responses(
        (status = 200, description = "Recurring transaction updated successfully", body = ApiResponse<RecurringTransactionResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Recurring transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_recurring_transaction(
    Path(recurring_transaction_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateRecurringTransactionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringTransactionResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering update_recurring_transaction function");
    debug!("Updating recurring transaction with ID: {}", recurring_transaction_id);

    // First, fetch the existing transaction
    let existing_transaction = match recurring_transaction::Entity::find_by_id(recurring_transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction)) => transaction,
        Ok(None) => {
            warn!("Recurring transaction with ID {} not found", recurring_transaction_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring transaction with id {} does not exist", recurring_transaction_id),
                    code: "RECURRING_TRANSACTION_NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Database error while fetching recurring transaction: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring transaction".to_string(),
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
    let mut update_model: recurring_transaction::ActiveModel = existing_transaction.into();

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
    if let Some(source_account_id) = request.source_account_id {
        update_model.source_account_id = Set(Some(source_account_id));
    }
    if let Some(ledger_name) = request.ledger_name {
        update_model.ledger_name = Set(Some(ledger_name));
    }

    match update_model.update(&state.db).await {
        Ok(updated_transaction) => {
            info!("Successfully updated recurring transaction with ID: {}", updated_transaction.id);
            let response = ApiResponse {
                data: RecurringTransactionResponse::from(updated_transaction),
                message: "Recurring transaction updated successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            error!("Failed to update recurring transaction: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update recurring transaction".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Delete a recurring transaction
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-transactions/{recurring_transaction_id}",
    tag = "recurring-transactions",
    responses(
        (status = 200, description = "Recurring transaction deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Recurring transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_recurring_transaction(
    Path(recurring_transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering delete_recurring_transaction function");
    debug!("Deleting recurring transaction with ID: {}", recurring_transaction_id);

    // First, check if the transaction exists
    match recurring_transaction::Entity::find_by_id(recurring_transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            // Transaction exists, proceed with deletion
            match recurring_transaction::Entity::delete_by_id(recurring_transaction_id)
                .exec(&state.db)
                .await
            {
                Ok(_) => {
                    info!("Successfully deleted recurring transaction with ID: {}", recurring_transaction_id);
                    let response = ApiResponse {
                        data: format!("Recurring transaction with id {} deleted successfully", recurring_transaction_id),
                        message: "Recurring transaction deleted successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(e) => {
                    error!("Failed to delete recurring transaction: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to delete recurring transaction".to_string(),
                            code: "DATABASE_ERROR".to_string(),
                            success: false,
                        }),
                    ))
                }
            }
        }
        Ok(None) => {
            warn!("Recurring transaction with ID {} not found", recurring_transaction_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring transaction with id {} does not exist", recurring_transaction_id),
                    code: "RECURRING_TRANSACTION_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while checking recurring transaction existence: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check recurring transaction existence".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}
