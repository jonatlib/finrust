use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::{one_off_transaction, account};
use model::transaction::{Tag, Transaction, TransactionGenerator};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, DbErr};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

/// Request body for creating a new one-off transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateTransactionRequest {
    /// Transaction name
    pub name: String,
    /// Transaction description
    pub description: Option<String>,
    /// Transaction amount (positive for income, negative for expense)
    pub amount: Decimal,
    /// Transaction date
    pub date: NaiveDate,
    /// Whether to include in statistics (default: true)
    pub include_in_statistics: Option<bool>,
    /// Target account ID
    pub target_account_id: i32,
    /// Source account ID for transfers
    pub source_account_id: Option<i32>,
    /// Ledger name for export
    pub ledger_name: Option<String>,
    /// Linked import ID to prevent duplication
    pub linked_import_id: Option<String>,
}

/// Request body for updating a transaction
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTransactionRequest {
    /// Transaction name
    pub name: Option<String>,
    /// Transaction description
    pub description: Option<String>,
    /// Transaction amount (positive for income, negative for expense)
    pub amount: Option<Decimal>,
    /// Transaction date
    pub date: Option<NaiveDate>,
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Target account ID
    pub target_account_id: Option<i32>,
    /// Source account ID for transfers
    pub source_account_id: Option<i32>,
    /// Ledger name for export
    pub ledger_name: Option<String>,
    /// Linked import ID to prevent duplication
    pub linked_import_id: Option<String>,
}

/// Tag information for API responses
#[derive(Debug, Serialize, ToSchema)]
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

/// Transaction response model
#[derive(Debug, Serialize, ToSchema)]
pub struct TransactionResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub include_in_statistics: bool,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
    pub linked_import_id: Option<String>,
    pub tags: Vec<TagInfo>,
}

impl From<one_off_transaction::Model> for TransactionResponse {
    fn from(model: one_off_transaction::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            amount: model.amount,
            date: model.date,
            include_in_statistics: model.include_in_statistics,
            target_account_id: model.target_account_id,
            source_account_id: model.source_account_id,
            ledger_name: model.ledger_name,
            linked_import_id: model.linked_import_id,
            tags: Vec::new(), // Will be populated by with_tags method
        }
    }
}

impl TransactionResponse {
    /// Create a TransactionResponse with tags fetched from the database
    pub async fn with_tags(
        model: one_off_transaction::Model,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Self, DbErr> {
        // Use the get_tag_for_transaction method from the Transaction trait
        let tags = model.get_tag_for_transaction(db, true).await;
        let tag_infos: Vec<TagInfo> = tags.into_iter().map(TagInfo::from).collect();
        
        let mut response = Self::from(model);
        response.tags = tag_infos;
        Ok(response)
    }
}

/// Create a new transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions",
    tag = "transactions",
    request_body = CreateTransactionRequest,
    responses(
        (status = 201, description = "Transaction created successfully", body = ApiResponse<TransactionResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_transaction(
    State(state): State<AppState>,
    Json(request): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TransactionResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_transaction function");
    debug!("Creating transaction with name: {}, amount: {}, target_account_id: {}", 
           request.name, request.amount, request.target_account_id);

    // Validate that the target account exists
    trace!("Validating target_account_id: {}", request.target_account_id);
    match account::Entity::find_by_id(request.target_account_id).one(&state.db).await {
        Ok(Some(_account)) => {
            debug!("Target account with ID {} found", request.target_account_id);
        }
        Ok(None) => {
            warn!("Attempted to create transaction with non-existent target_account_id: {}", request.target_account_id);
            let error_response = ErrorResponse {
                error: format!("Target account with id {} does not exist", request.target_account_id),
                code: "INVALID_TARGET_ACCOUNT_ID".to_string(),
                success: false,
            };
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
        Err(db_error) => {
            error!("Database error while validating target_account_id {}: {}", request.target_account_id, db_error);
            let error_response = ErrorResponse {
                error: "Internal server error while validating target account".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

    // Validate source account if provided
    if let Some(source_account_id) = request.source_account_id {
        trace!("Validating source_account_id: {}", source_account_id);
        match account::Entity::find_by_id(source_account_id).one(&state.db).await {
            Ok(Some(_account)) => {
                debug!("Source account with ID {} found", source_account_id);
            }
            Ok(None) => {
                warn!("Attempted to create transaction with non-existent source_account_id: {}", source_account_id);
                let error_response = ErrorResponse {
                    error: format!("Source account with id {} does not exist", source_account_id),
                    code: "INVALID_SOURCE_ACCOUNT_ID".to_string(),
                    success: false,
                };
                return Err((StatusCode::BAD_REQUEST, Json(error_response)));
            }
            Err(db_error) => {
                error!("Database error while validating source_account_id {}: {}", source_account_id, db_error);
                let error_response = ErrorResponse {
                    error: "Internal server error while validating source account".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                };
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
            }
        }
    }

    let new_transaction = one_off_transaction::ActiveModel {
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        amount: Set(request.amount),
        date: Set(request.date),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        target_account_id: Set(request.target_account_id),
        source_account_id: Set(request.source_account_id),
        ledger_name: Set(request.ledger_name.clone()),
        linked_import_id: Set(request.linked_import_id.clone()),
        ..Default::default()
    };

    trace!("Attempting to insert new transaction into database");
    match new_transaction.insert(&state.db).await {
        Ok(transaction_model) => {
            info!("Transaction created successfully with ID: {}, name: {}, amount: {}", 
                  transaction_model.id, transaction_model.name, transaction_model.amount);
            
            match TransactionResponse::with_tags(transaction_model.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Transaction created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
                Err(tag_error) => {
                    error!("Failed to fetch tags for transaction: {}", tag_error);
                    // Fall back to response without tags
                    let response = ApiResponse {
                        data: TransactionResponse::from(transaction_model),
                        message: "Transaction created successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
            }
        }
        Err(db_error) => {
            error!("Failed to create transaction '{}' for target account {}: {}", 
                   request.name, request.target_account_id, db_error);

            // Handle specific database errors
            let error_response = match db_error {
                DbErr::Exec(ref exec_err) => {
                    // Check for foreign key constraint violations or other specific errors
                    let error_msg = exec_err.to_string().to_lowercase();
                    if error_msg.contains("foreign key") || error_msg.contains("constraint") {
                        ErrorResponse {
                            error: "Invalid account reference in transaction".to_string(),
                            code: "FOREIGN_KEY_VIOLATION".to_string(),
                            success: false,
                        }
                    } else {
                        ErrorResponse {
                            error: "Failed to create transaction due to database constraint".to_string(),
                            code: "DATABASE_CONSTRAINT_ERROR".to_string(),
                            success: false,
                        }
                    }
                }
                _ => ErrorResponse {
                    error: "Internal server error while creating transaction".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }
            };

            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Get all transactions
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "transactions",
    responses(
        (status = 200, description = "Transactions retrieved successfully", body = ApiResponse<Vec<TransactionResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_transactions(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TransactionResponse>>>, StatusCode> {
    trace!("Entering get_transactions function");
    debug!("Fetching all transactions from database");

    match one_off_transaction::Entity::find().all(&state.db).await {
        Ok(transactions) => {
            let transaction_count = transactions.len();
            debug!("Retrieved {} transactions from database", transaction_count);

            let mut transaction_responses = Vec::new();
            for transaction in transactions {
                match TransactionResponse::with_tags(transaction.clone(), &state.db).await {
                    Ok(response) => transaction_responses.push(response),
                    Err(tag_error) => {
                        warn!("Failed to fetch tags for transaction {}: {}", transaction.id, tag_error);
                        transaction_responses.push(TransactionResponse::from(transaction));
                    }
                }
            }

            info!("Successfully retrieved {} transactions", transaction_count);
            let response = ApiResponse {
                data: transaction_responses,
                message: "Transactions retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to retrieve transactions from database: {}", db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get transactions for a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/transactions",
    tag = "transactions",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account transactions retrieved successfully", body = ApiResponse<Vec<TransactionResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_transactions(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TransactionResponse>>>, StatusCode> {
    trace!("Entering get_account_transactions function for account_id: {}", account_id);
    debug!("Fetching transactions for account ID: {}", account_id);

    use sea_orm::{ColumnTrait, Condition, QueryFilter};

    // Find transactions where the account is either target or source
    let condition = Condition::any()
        .add(one_off_transaction::Column::TargetAccountId.eq(account_id))
        .add(one_off_transaction::Column::SourceAccountId.eq(account_id));

    trace!("Executing query to find transactions for account {}", account_id);
    match one_off_transaction::Entity::find()
        .filter(condition)
        .all(&state.db)
        .await
    {
        Ok(transactions) => {
            let transaction_count = transactions.len();
            debug!("Retrieved {} transactions for account ID: {}", transaction_count, account_id);

            let mut transaction_responses = Vec::new();
            for transaction in transactions {
                match TransactionResponse::with_tags(transaction.clone(), &state.db).await {
                    Ok(response) => transaction_responses.push(response),
                    Err(tag_error) => {
                        warn!("Failed to fetch tags for transaction {}: {}", transaction.id, tag_error);
                        transaction_responses.push(TransactionResponse::from(transaction));
                    }
                }
            }

            info!("Successfully retrieved {} transactions for account ID: {}", transaction_count, account_id);
            let response = ApiResponse {
                data: transaction_responses,
                message: "Account transactions retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to retrieve transactions for account ID {}: {}", account_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific transaction by ID
#[utoipa::path(
    get,
    path = "/api/v1/transactions/{transaction_id}",
    tag = "transactions",
    params(
        ("transaction_id" = i32, Path, description = "Transaction ID"),
    ),
    responses(
        (status = 200, description = "Transaction retrieved successfully", body = ApiResponse<TransactionResponse>),
        (status = 404, description = "Transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TransactionResponse>>, StatusCode> {
    trace!("Entering get_transaction function for transaction_id: {}", transaction_id);
    debug!("Fetching transaction with ID: {}", transaction_id);

    match one_off_transaction::Entity::find_by_id(transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction_model)) => {
            info!("Successfully retrieved transaction with ID: {}, name: {}", 
                  transaction_model.id, transaction_model.name);
            
            match TransactionResponse::with_tags(transaction_model.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Transaction retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok(Json(response))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for transaction {}: {}", transaction_model.id, tag_error);
                    let response = ApiResponse {
                        data: TransactionResponse::from(transaction_model),
                        message: "Transaction retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok(Json(response))
                }
            }
        }
        Ok(None) => {
            warn!("Transaction with ID {} not found", transaction_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(db_error) => {
            error!("Failed to retrieve transaction with ID {}: {}", transaction_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update a transaction
#[utoipa::path(
    put,
    path = "/api/v1/transactions/{transaction_id}",
    tag = "transactions",
    params(
        ("transaction_id" = i32, Path, description = "Transaction ID"),
    ),
    request_body = UpdateTransactionRequest,
    responses(
        (status = 200, description = "Transaction updated successfully", body = ApiResponse<TransactionResponse>),
        (status = 404, description = "Transaction not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateTransactionRequest>,
) -> Result<Json<ApiResponse<TransactionResponse>>, StatusCode> {
    trace!("Entering update_transaction function for transaction_id: {}", transaction_id);
    debug!("Updating transaction with ID: {}", transaction_id);

    // First, find the existing transaction
    trace!("Looking up existing transaction with ID: {}", transaction_id);
    let existing_transaction = match one_off_transaction::Entity::find_by_id(transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction)) => {
            debug!("Found existing transaction: {}", transaction.name);
            transaction
        }
        Ok(None) => {
            warn!("Transaction with ID {} not found for update", transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(db_error) => {
            error!("Failed to lookup transaction with ID {} for update: {}", transaction_id, db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create active model for update
    let mut transaction_active: one_off_transaction::ActiveModel = existing_transaction.into();
    let mut updated_fields = Vec::new();

    // Update only provided fields
    if let Some(name) = request.name {
        debug!("Updating transaction name to: {}", name);
        transaction_active.name = Set(name.clone());
        updated_fields.push(format!("name: {}", name));
    }
    if let Some(description) = request.description {
        debug!("Updating transaction description");
        transaction_active.description = Set(Some(description.clone()));
        updated_fields.push(format!("description: {:?}", description));
    }
    if let Some(amount) = request.amount {
        debug!("Updating transaction amount to: {}", amount);
        transaction_active.amount = Set(amount);
        updated_fields.push(format!("amount: {}", amount));
    }
    if let Some(date) = request.date {
        debug!("Updating transaction date to: {}", date);
        transaction_active.date = Set(date);
        updated_fields.push(format!("date: {}", date));
    }
    if let Some(include_in_statistics) = request.include_in_statistics {
        debug!("Updating transaction include_in_statistics to: {}", include_in_statistics);
        transaction_active.include_in_statistics = Set(include_in_statistics);
        updated_fields.push(format!("include_in_statistics: {}", include_in_statistics));
    }
    if let Some(target_account_id) = request.target_account_id {
        debug!("Updating transaction target_account_id to: {}", target_account_id);
        transaction_active.target_account_id = Set(target_account_id);
        updated_fields.push(format!("target_account_id: {}", target_account_id));
    }
    if let Some(source_account_id) = request.source_account_id {
        debug!("Updating transaction source_account_id to: {:?}", source_account_id);
        transaction_active.source_account_id = Set(Some(source_account_id));
        updated_fields.push(format!("source_account_id: {:?}", source_account_id));
    }
    if let Some(ledger_name) = request.ledger_name {
        debug!("Updating transaction ledger_name to: {:?}", ledger_name);
        transaction_active.ledger_name = Set(Some(ledger_name.clone()));
        updated_fields.push(format!("ledger_name: {:?}", ledger_name));
    }
    if let Some(linked_import_id) = request.linked_import_id {
        debug!("Updating transaction linked_import_id to: {:?}", linked_import_id);
        transaction_active.linked_import_id = Set(Some(linked_import_id.clone()));
        updated_fields.push(format!("linked_import_id: {:?}", linked_import_id));
    }

    if updated_fields.is_empty() {
        debug!("No fields to update for transaction ID: {}", transaction_id);
    } else {
        debug!("Updating fields: {}", updated_fields.join(", "));
    }

    trace!("Attempting to update transaction in database");
    match transaction_active.update(&state.db).await {
        Ok(updated_transaction) => {
            info!("Transaction with ID {} updated successfully. Updated fields: {}", 
                  transaction_id, if updated_fields.is_empty() { "none".to_string() } else { updated_fields.join(", ") });
            
            match TransactionResponse::with_tags(updated_transaction.clone(), &state.db).await {
                Ok(transaction_response) => {
                    let response = ApiResponse {
                        data: transaction_response,
                        message: "Transaction updated successfully".to_string(),
                        success: true,
                    };
                    Ok(Json(response))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for updated transaction {}: {}", updated_transaction.id, tag_error);
                    let response = ApiResponse {
                        data: TransactionResponse::from(updated_transaction),
                        message: "Transaction updated successfully".to_string(),
                        success: true,
                    };
                    Ok(Json(response))
                }
            }
        }
        Err(db_error) => {
            error!("Failed to update transaction with ID {}: {}", transaction_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete a transaction
#[utoipa::path(
    delete,
    path = "/api/v1/transactions/{transaction_id}",
    tag = "transactions",
    params(
        ("transaction_id" = i32, Path, description = "Transaction ID"),
    ),
    responses(
        (status = 200, description = "Transaction deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    trace!("Entering delete_transaction function for transaction_id: {}", transaction_id);
    debug!("Attempting to delete transaction with ID: {}", transaction_id);

    match one_off_transaction::Entity::delete_by_id(transaction_id)
        .exec(&state.db)
        .await
    {
        Ok(delete_result) => {
            debug!("Delete operation completed. Rows affected: {}", delete_result.rows_affected);
            if delete_result.rows_affected > 0 {
                info!("Transaction with ID {} deleted successfully", transaction_id);
                let response = ApiResponse {
                    data: format!("Transaction {} deleted", transaction_id),
                    message: "Transaction deleted successfully".to_string(),
                    success: true,
                };
                Ok(Json(response))
            } else {
                warn!("Transaction with ID {} not found for deletion (no rows affected)", transaction_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(db_error) => {
            error!("Failed to delete transaction with ID {}: {}", transaction_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
