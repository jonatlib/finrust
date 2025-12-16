use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use chrono::NaiveDate;
use model::entities::{recurring_transaction, recurring_transaction_instance};
use model::transaction::{Tag, TransactionGenerator};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, trace, warn};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Tag information for API responses
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TagInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

impl From<Tag> for TagInfo {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name,
            description: tag.description,
        }
    }
}

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
    /// Optional category ID
    pub category_id: Option<i32>,
    /// Scenario ID for what-if analysis (optional)
    pub scenario_id: Option<i32>,
    /// Whether this is a simulated transaction (default: false)
    pub is_simulated: Option<bool>,
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
    /// Optional category ID
    pub category_id: Option<i32>,
    /// Scenario ID for what-if analysis (optional)
    pub scenario_id: Option<i32>,
    /// Whether this is a simulated transaction (default: false)
    pub is_simulated: Option<bool>,
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
    pub category_id: Option<i32>,
    pub tags: Vec<TagInfo>,
    pub scenario_id: Option<i32>,
    pub is_simulated: bool,
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
            category_id: model.category_id,
            tags: Vec::new(), // Will be populated by with_tags method
            scenario_id: model.scenario_id,
            is_simulated: model.is_simulated,
        }
    }
}

impl RecurringTransactionResponse {
    /// Create a RecurringTransactionResponse with tags fetched from the database
    pub async fn with_tags(
        model: recurring_transaction::Model,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Self, sea_orm::DbErr> {
        // Use the get_tag_for_transaction method from the TransactionGenerator trait
        let tags = model.get_tag_for_transaction(db, true).await;
        let tag_infos: Vec<TagInfo> = tags.into_iter().map(TagInfo::from).collect();

        let mut response = Self::from(model);
        response.tags = tag_infos;
        Ok(response)
    }
}

/// Query parameters for listing recurring transactions
#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct RecurringTransactionQuery {
    /// Page number (default: 1)
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    /// Page size (default: 50)
    #[validate(range(min = 1, max = 1000))]
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
    pub recurring_transaction_name: Option<String>,
    pub target_account_id: Option<i32>,
    pub target_account_name: Option<String>,
    pub source_account_id: Option<i32>,
    pub source_account_name: Option<String>,
    pub status: String,
    pub due_date: NaiveDate,
    pub expected_amount: Decimal,
    pub paid_date: Option<NaiveDate>,
    pub paid_amount: Option<Decimal>,
    pub reconciled_imported_transaction_id: Option<i32>,
    pub tags: Vec<TagInfo>,
}

impl From<recurring_transaction_instance::Model> for RecurringInstanceResponse {
    fn from(model: recurring_transaction_instance::Model) -> Self {
        Self {
            id: model.id,
            recurring_transaction_id: model.recurring_transaction_id,
            recurring_transaction_name: None,
            target_account_id: None,
            target_account_name: None,
            source_account_id: None,
            source_account_name: None,
            status: format!("{:?}", model.status),
            due_date: model.due_date,
            expected_amount: model.expected_amount,
            paid_date: model.paid_date,
            paid_amount: model.paid_amount,
            reconciled_imported_transaction_id: model.reconciled_imported_transaction_id,
            tags: Vec::new(), // Will be populated by with_tags method
        }
    }
}

impl RecurringInstanceResponse {
    /// Create a RecurringInstanceResponse with tags fetched from the parent recurring transaction
    pub async fn with_tags(
        model: recurring_transaction_instance::Model,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Self, sea_orm::DbErr> {
        use sea_orm::EntityTrait;
        use model::entities::account;

        // Fetch the parent recurring transaction to get its tags
        let parent_transaction = recurring_transaction::Entity::find_by_id(model.recurring_transaction_id)
            .one(db)
            .await?;

        let mut response = Self::from(model);

        if let Some(parent) = parent_transaction {
            // Get transaction name
            response.recurring_transaction_name = Some(parent.name.clone());
            response.target_account_id = Some(parent.target_account_id);
            response.source_account_id = parent.source_account_id;

            // Fetch target account name
            if let Ok(Some(target_account)) = account::Entity::find_by_id(parent.target_account_id).one(db).await {
                response.target_account_name = Some(target_account.name);
            }

            // Fetch source account name if present
            if let Some(source_id) = parent.source_account_id {
                if let Ok(Some(source_account)) = account::Entity::find_by_id(source_id).one(db).await {
                    response.source_account_name = Some(source_account.name);
                }
            }

            // Use the get_tag_for_transaction method from the TransactionGenerator trait
            let tags = parent.get_tag_for_transaction(db, true).await;
            response.tags = tags.into_iter().map(TagInfo::from).collect();
        }

        Ok(response)
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

            match RecurringInstanceResponse::with_tags(instance.clone(), &state.db).await {
                Ok(instance_response) => {
                    let response = ApiResponse {
                        data: instance_response,
                        message: "Recurring transaction instance created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
                Err(tag_error) => {
                    error!("Failed to fetch tags for recurring transaction instance: {}", tag_error);
                    // Fall back to response without tags
                    let response = ApiResponse {
                        data: RecurringInstanceResponse::from(instance),
                        message: "Recurring transaction instance created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
            }
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
        category_id: Set(request.category_id),
        scenario_id: Set(request.scenario_id),
        is_simulated: Set(request.is_simulated.unwrap_or(false)),
        ..Default::default()
    };

    match new_transaction.insert(&state.db).await {
        Ok(transaction) => {
            info!("Successfully created recurring transaction with ID: {}", transaction.id);

            match RecurringTransactionResponse::with_tags(transaction.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Recurring transaction created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
                Err(tag_error) => {
                    error!("Failed to fetch tags for recurring transaction: {}", tag_error);
                    // Fall back to response without tags
                    let response = ApiResponse {
                        data: RecurringTransactionResponse::from(transaction),
                        message: "Recurring transaction created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
            }
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
    Valid(Query(query)): Valid<Query<RecurringTransactionQuery>>,
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

            let mut response_data = Vec::new();
            for transaction in transactions {
                match RecurringTransactionResponse::with_tags(transaction.clone(), &state.db).await {
                    Ok(response) => response_data.push(response),
                    Err(tag_error) => {
                        warn!("Failed to fetch tags for recurring transaction {}: {}", transaction.id, tag_error);
                        response_data.push(RecurringTransactionResponse::from(transaction));
                    }
                }
            }

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

            match RecurringTransactionResponse::with_tags(transaction.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Recurring transaction retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for recurring transaction {}: {}", transaction.id, tag_error);
                    let response = ApiResponse {
                        data: RecurringTransactionResponse::from(transaction),
                        message: "Recurring transaction retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
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
    if let Some(category_id) = request.category_id {
        update_model.category_id = Set(Some(category_id));
    }
    if let Some(scenario_id) = request.scenario_id {
        update_model.scenario_id = Set(Some(scenario_id));
    }
    if let Some(is_simulated) = request.is_simulated {
        update_model.is_simulated = Set(is_simulated);
    }

    match update_model.update(&state.db).await {
        Ok(updated_transaction) => {
            info!("Successfully updated recurring transaction with ID: {}", updated_transaction.id);

            match RecurringTransactionResponse::with_tags(updated_transaction.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Recurring transaction updated successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for updated recurring transaction {}: {}", updated_transaction.id, tag_error);
                    let response = ApiResponse {
                        data: RecurringTransactionResponse::from(updated_transaction),
                        message: "Recurring transaction updated successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
            }
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

/// Missing instance information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MissingInstanceInfo {
    pub recurring_transaction_id: i32,
    pub recurring_transaction_name: String,
    pub due_date: NaiveDate,
    pub expected_amount: Decimal,
    /// If true, instance exists but is in Pending status
    pub is_pending: bool,
    /// If the instance exists (pending), this is its ID
    pub instance_id: Option<i32>,
}

/// Query parameters for getting missing instances
#[derive(Debug, Deserialize, IntoParams, Validate)]
pub struct MissingInstancesQuery {
    /// Start date for the range (defaults to 6 months ago)
    pub start_date: Option<NaiveDate>,
    /// End date for the range (defaults to today)
    pub end_date: Option<NaiveDate>,
    /// Optional recurring transaction ID to filter by
    pub recurring_transaction_id: Option<i32>,
}

/// Get missing instances for recurring transactions
#[utoipa::path(
    get,
    path = "/api/v1/recurring-transactions/missing-instances",
    tag = "recurring-transactions",
    params(MissingInstancesQuery),
    responses(
        (status = 200, description = "Missing instances retrieved successfully", body = ApiResponse<Vec<MissingInstanceInfo>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument(skip(state))]
pub async fn get_missing_instances(
    Valid(Query(query)): Valid<Query<MissingInstancesQuery>>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<MissingInstanceInfo>>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Fetching missing instances with query: {:?}", query);

    let today = chrono::Local::now().date_naive();
    let start_date = query.start_date.unwrap_or_else(|| today - chrono::Duration::days(31 * 16));
    let end_date = query.end_date.unwrap_or(today + chrono::Duration::days(5));

    // Fetch recurring transactions
    let recurring_transactions = if let Some(rt_id) = query.recurring_transaction_id {
        // Fetch specific recurring transaction
        match recurring_transaction::Entity::find_by_id(rt_id).one(&state.db).await {
            Ok(Some(rt)) => vec![rt],
            Ok(None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("Recurring transaction with id {} not found", rt_id),
                        code: "NOT_FOUND".to_string(),
                        success: false,
                    }),
                ));
            }
            Err(e) => {
                error!("Database error while fetching recurring transaction: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to fetch recurring transaction".to_string(),
                        code: "DATABASE_ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    } else {
        // Fetch all recurring transactions
        match recurring_transaction::Entity::find().all(&state.db).await {
            Ok(rts) => rts,
            Err(e) => {
                error!("Database error while fetching recurring transactions: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to fetch recurring transactions".to_string(),
                        code: "DATABASE_ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    };

    let mut missing_instances = Vec::new();

    // For each recurring transaction, generate expected dates and check for missing instances
    for rt in recurring_transactions {
        // Use the transaction generator to get all expected transaction dates
        let transactions = rt.generate_transactions(start_date, end_date, today, &state.db).await;

        // Get the unique dates from the transactions (for the target account)
        let mut expected_dates: std::collections::HashSet<NaiveDate> = std::collections::HashSet::new();
        for transaction in transactions {
            if transaction.account() == rt.target_account_id {
                expected_dates.insert(transaction.date());
            }
        }

        // Fetch existing instances for this recurring transaction in the date range
        let existing_instances = match recurring_transaction_instance::Entity::find()
            .filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(rt.id))
            .filter(recurring_transaction_instance::Column::DueDate.gte(start_date))
            .filter(recurring_transaction_instance::Column::DueDate.lte(end_date))
            .all(&state.db)
            .await
        {
            Ok(instances) => instances,
            Err(e) => {
                error!("Database error while fetching instances: {}", e);
                continue; // Skip this recurring transaction
            }
        };

        // Build a map of existing instance dates to their status and ID
        let existing_map: std::collections::HashMap<NaiveDate, (recurring_transaction_instance::InstanceStatus, i32)> = existing_instances
            .iter()
            .map(|instance| (instance.due_date, (instance.status.clone(), instance.id)))
            .collect();

        // Find missing dates or pending instances (expected but not paid/skipped)
        for expected_date in expected_dates {
            if expected_date <= today {
                match existing_map.get(&expected_date) {
                    None => {
                        // Truly missing - no instance exists
                        missing_instances.push(MissingInstanceInfo {
                            recurring_transaction_id: rt.id,
                            recurring_transaction_name: rt.name.clone(),
                            due_date: expected_date,
                            expected_amount: rt.amount,
                            is_pending: false,
                            instance_id: None,
                        });
                    }
                    Some((recurring_transaction_instance::InstanceStatus::Pending, id)) => {
                        // Instance exists but is pending
                        missing_instances.push(MissingInstanceInfo {
                            recurring_transaction_id: rt.id,
                            recurring_transaction_name: rt.name.clone(),
                            due_date: expected_date,
                            expected_amount: rt.amount,
                            is_pending: true,
                            instance_id: Some(*id),
                        });
                    }
                    Some((recurring_transaction_instance::InstanceStatus::Paid, _)) => {
                        // Instance is paid, skip it
                    }
                    Some((recurring_transaction_instance::InstanceStatus::Skipped, _)) => {
                        // Instance is skipped, skip it
                    }
                }
            }
        }
    }

    // Sort by due date
    missing_instances.sort_by(|a, b| a.due_date.cmp(&b.due_date));

    info!("Found {} missing instances", missing_instances.len());

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            data: missing_instances,
            message: "Missing instances retrieved successfully".to_string(),
            success: true,
        }),
    ))
}

/// Request body for bulk creating/updating instances
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct BulkCreateInstancesRequest {
    /// List of instances to create/update
    pub instances: Vec<BulkInstanceItem>,
    /// Whether to mark instances as paid (true) or pending (false)
    pub mark_as_paid: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkInstanceItem {
    pub recurring_transaction_id: i32,
    pub due_date: NaiveDate,
    /// If provided, this is an existing instance ID to update instead of create
    pub instance_id: Option<i32>,
}

/// Bulk create or update recurring transaction instances
#[utoipa::path(
    post,
    path = "/api/v1/recurring-transactions/bulk-create-instances",
    tag = "recurring-transactions",
    request_body = BulkCreateInstancesRequest,
    responses(
        (status = 200, description = "Instances created/updated successfully", body = ApiResponse<BulkCreateInstancesResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument(skip(state))]
pub async fn bulk_create_instances(
    State(state): State<AppState>,
    Valid(Json(request)): Valid<Json<BulkCreateInstancesRequest>>,
) -> Result<(StatusCode, Json<ApiResponse<BulkCreateInstancesResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Bulk creating/updating {} instances", request.instances.len());

    let mut created_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;

    for item in request.instances {
        // If instance_id is provided, update existing instance
        if let Some(instance_id) = item.instance_id {
            // Only update if marking as paid
            if request.mark_as_paid {
                match recurring_transaction_instance::Entity::find_by_id(instance_id).one(&state.db).await {
                    Ok(Some(instance)) => {
                        let mut active_model: recurring_transaction_instance::ActiveModel = instance.into();
                        active_model.status = Set(recurring_transaction_instance::InstanceStatus::Paid);
                        active_model.paid_date = Set(Some(item.due_date));
                        active_model.paid_amount = Set(Some(active_model.expected_amount.clone().unwrap()));

                        match active_model.update(&state.db).await {
                            Ok(_) => {
                                updated_count += 1;
                            }
                            Err(e) => {
                                error!("Failed to update instance {}: {}", instance_id, e);
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("Instance {} not found for update", instance_id);
                    }
                    Err(e) => {
                        error!("Database error while fetching instance {}: {}", instance_id, e);
                    }
                }
            } else {
                // If marking as pending and instance already exists as pending, skip it
                skipped_count += 1;
            }
        } else {
            // Create new instance
            // First, fetch the recurring transaction to get the amount
            let recurring_transaction = match recurring_transaction::Entity::find_by_id(item.recurring_transaction_id).one(&state.db).await {
                Ok(Some(rt)) => rt,
                Ok(None) => {
                    warn!("Recurring transaction {} not found", item.recurring_transaction_id);
                    continue;
                }
                Err(e) => {
                    error!("Database error while fetching recurring transaction: {}", e);
                    continue;
                }
            };

            let new_instance = recurring_transaction_instance::ActiveModel {
                recurring_transaction_id: Set(item.recurring_transaction_id),
                status: Set(if request.mark_as_paid {
                    recurring_transaction_instance::InstanceStatus::Paid
                } else {
                    recurring_transaction_instance::InstanceStatus::Pending
                }),
                due_date: Set(item.due_date),
                expected_amount: Set(recurring_transaction.amount),
                paid_date: Set(if request.mark_as_paid { Some(item.due_date) } else { None }),
                paid_amount: Set(if request.mark_as_paid { Some(recurring_transaction.amount) } else { None }),
                reconciled_imported_transaction_id: Set(None),
                ..Default::default()
            };

            match new_instance.insert(&state.db).await {
                Ok(_) => {
                    created_count += 1;
                }
                Err(e) => {
                    error!("Failed to create instance: {}", e);
                }
            }
        }
    }

    info!("Bulk operation completed: {} created, {} updated, {} skipped", created_count, updated_count, skipped_count);

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            data: BulkCreateInstancesResponse {
                created_count,
                updated_count,
                skipped_count,
            },
            message: format!("Processed {} instances: {} created, {} updated, {} skipped",
                             created_count + updated_count + skipped_count, created_count, updated_count, skipped_count),
            success: true,
        }),
    ))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkCreateInstancesResponse {
    pub created_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
}
