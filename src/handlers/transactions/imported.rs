use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::{imported_transaction, account};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::{ToSchema, IntoParams};

/// Request body for creating a new imported transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateImportedTransactionRequest {
    /// Account ID this transaction was imported for
    pub account_id: i32,
    /// Transaction date from the import file
    pub date: NaiveDate,
    /// Transaction description from the import file
    pub description: String,
    /// Transaction amount
    pub amount: Decimal,
    /// Unique hash to prevent duplicate imports
    pub import_hash: String,
    /// Raw transaction data as JSON for auditing
    pub raw_data: Option<serde_json::Value>,
}

/// Request body for updating an imported transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateImportedTransactionRequest {
    /// Transaction date
    pub date: Option<NaiveDate>,
    /// Transaction description
    pub description: Option<String>,
    /// Transaction amount
    pub amount: Option<Decimal>,
    /// Raw transaction data as JSON
    pub raw_data: Option<serde_json::Value>,
}

/// Request body for reconciling an imported transaction
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconcileImportedTransactionRequest {
    /// Type of transaction to reconcile with ("OneOff", "Recurring", "RecurringIncome", "RecurringInstance")
    pub transaction_type: String,
    /// ID of the transaction to reconcile with
    pub transaction_id: i32,
}

/// Imported transaction response model
#[derive(Debug, Serialize, ToSchema)]
pub struct ImportedTransactionResponse {
    pub id: i32,
    pub account_id: i32,
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub import_hash: String,
    pub raw_data: Option<serde_json::Value>,
    pub reconciled_transaction_type: Option<String>,
    pub reconciled_transaction_id: Option<i32>,
    pub reconciled_transaction_info: Option<ReconciledTransactionInfo>,
}

/// Information about the reconciled transaction
#[derive(Debug, Serialize, ToSchema)]
pub struct ReconciledTransactionInfo {
    pub transaction_type: String,
    pub transaction_id: i32,
}

/// Query parameters for filtering imported transactions
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ImportedTransactionQuery {
    /// Filter by account ID
    pub account_id: Option<i32>,
    /// Filter by reconciliation status
    pub reconciled: Option<bool>,
    /// Filter by date range start
    pub date_from: Option<NaiveDate>,
    /// Filter by date range end
    pub date_to: Option<NaiveDate>,
}

impl From<imported_transaction::Model> for ImportedTransactionResponse {
    fn from(model: imported_transaction::Model) -> Self {
        let reconciled_transaction_info = model.get_reconciled_transaction_type().map(|rt| {
            ReconciledTransactionInfo {
                transaction_type: match rt {
                    imported_transaction::ReconciledTransactionType::OneOff(_) => "OneOff".to_string(),
                    imported_transaction::ReconciledTransactionType::Recurring(_) => "Recurring".to_string(),
                    imported_transaction::ReconciledTransactionType::RecurringIncome(_) => "RecurringIncome".to_string(),
                    imported_transaction::ReconciledTransactionType::RecurringInstance(_) => "RecurringInstance".to_string(),
                },
                transaction_id: match rt {
                    imported_transaction::ReconciledTransactionType::OneOff(id) => id,
                    imported_transaction::ReconciledTransactionType::Recurring(id) => id,
                    imported_transaction::ReconciledTransactionType::RecurringIncome(id) => id,
                    imported_transaction::ReconciledTransactionType::RecurringInstance(id) => id,
                },
            }
        });

        let reconciled_transaction_type = model.reconciled_transaction_type.map(|rt| {
            match rt {
                imported_transaction::ReconciledTransactionEntityType::OneOff => "OneOff".to_string(),
                imported_transaction::ReconciledTransactionEntityType::Recurring => "Recurring".to_string(),
                imported_transaction::ReconciledTransactionEntityType::RecurringIncome => "RecurringIncome".to_string(),
                imported_transaction::ReconciledTransactionEntityType::RecurringInstance => "RecurringInstance".to_string(),
            }
        });

        Self {
            id: model.id,
            account_id: model.account_id,
            date: model.date,
            description: model.description,
            amount: model.amount,
            import_hash: model.import_hash,
            raw_data: model.raw_data.map(|json| json.into()),
            reconciled_transaction_type,
            reconciled_transaction_id: model.reconciled_transaction_id,
            reconciled_transaction_info,
        }
    }
}

/// Create a new imported transaction
#[utoipa::path(
    post,
    path = "/api/v1/imported-transactions",
    tag = "imported-transactions",
    request_body = CreateImportedTransactionRequest,
    responses(
        (status = 201, description = "Imported transaction created successfully", body = ApiResponse<ImportedTransactionResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Duplicate import hash", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_imported_transaction(
    State(state): State<AppState>,
    Json(request): Json<CreateImportedTransactionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ImportedTransactionResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_imported_transaction function");
    debug!("Creating imported transaction for account_id: {}, amount: {}, import_hash: {}", 
           request.account_id, request.amount, request.import_hash);

    // Validate that the account exists
    trace!("Validating account_id: {}", request.account_id);
    match account::Entity::find_by_id(request.account_id).one(&state.db).await {
        Ok(Some(_account)) => {
            debug!("Account with ID {} found", request.account_id);
        }
        Ok(None) => {
            warn!("Attempted to create imported transaction with non-existent account_id: {}", request.account_id);
            let error_response = ErrorResponse {
                error: format!("Account with id {} does not exist", request.account_id),
                code: "INVALID_ACCOUNT_ID".to_string(),
                success: false,
            };
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
        Err(e) => {
            error!("Database error while validating account_id {}: {}", request.account_id, e);
            let error_response = ErrorResponse {
                error: "Database error occurred while validating account".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

    // Check for duplicate import hash
    trace!("Checking for duplicate import_hash: {}", request.import_hash);
    match imported_transaction::Entity::find()
        .filter(imported_transaction::Column::ImportHash.eq(&request.import_hash))
        .one(&state.db)
        .await
    {
        Ok(Some(_existing)) => {
            warn!("Attempted to create imported transaction with duplicate import_hash: {}", request.import_hash);
            let error_response = ErrorResponse {
                error: format!("Imported transaction with hash {} already exists", request.import_hash),
                code: "DUPLICATE_IMPORT_HASH".to_string(),
                success: false,
            };
            return Err((StatusCode::CONFLICT, Json(error_response)));
        }
        Ok(None) => {
            debug!("Import hash {} is unique", request.import_hash);
        }
        Err(e) => {
            error!("Database error while checking import_hash {}: {}", request.import_hash, e);
            let error_response = ErrorResponse {
                error: "Database error occurred while checking for duplicates".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

    // Create the imported transaction
    let new_imported_transaction = imported_transaction::ActiveModel {
        account_id: Set(request.account_id),
        date: Set(request.date),
        description: Set(request.description),
        amount: Set(request.amount),
        import_hash: Set(request.import_hash),
        raw_data: Set(request.raw_data.map(sea_orm::JsonValue::from)),
        reconciled_transaction_type: Set(None),
        reconciled_transaction_id: Set(None),
        ..Default::default()
    };

    trace!("Attempting to save imported transaction to database");
    match new_imported_transaction.insert(&state.db).await {
        Ok(imported_transaction) => {
            info!("Successfully created imported transaction with id: {}", imported_transaction.id);
            let response = ImportedTransactionResponse::from(imported_transaction);
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse {
                    data: response,
                    message: "Imported transaction created successfully".to_string(),
                    success: true,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to create imported transaction: {}", e);
            let error_response = ErrorResponse {
                error: "Failed to create imported transaction".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Get all imported transactions with optional filtering
#[utoipa::path(
    get,
    path = "/api/v1/imported-transactions",
    tag = "imported-transactions",
    params(ImportedTransactionQuery),
    responses(
        (status = 200, description = "List of imported transactions", body = ApiResponse<Vec<ImportedTransactionResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_imported_transactions(
    State(state): State<AppState>,
    Query(query): Query<ImportedTransactionQuery>,
) -> Result<Json<ApiResponse<Vec<ImportedTransactionResponse>>>, StatusCode> {
    trace!("Entering get_imported_transactions function");
    debug!("Getting imported transactions with query: {:?}", query);

    let mut query_builder = imported_transaction::Entity::find();

    // Apply filters
    if let Some(account_id) = query.account_id {
        query_builder = query_builder.filter(imported_transaction::Column::AccountId.eq(account_id));
    }

    if let Some(reconciled) = query.reconciled {
        if reconciled {
            query_builder = query_builder.filter(imported_transaction::Column::ReconciledTransactionId.is_not_null());
        } else {
            query_builder = query_builder.filter(imported_transaction::Column::ReconciledTransactionId.is_null());
        }
    }

    if let Some(date_from) = query.date_from {
        query_builder = query_builder.filter(imported_transaction::Column::Date.gte(date_from));
    }

    if let Some(date_to) = query.date_to {
        query_builder = query_builder.filter(imported_transaction::Column::Date.lte(date_to));
    }

    trace!("Executing database query for imported transactions");
    match query_builder.all(&state.db).await {
        Ok(imported_transactions) => {
            info!("Successfully retrieved {} imported transactions", imported_transactions.len());
            let responses: Vec<ImportedTransactionResponse> = imported_transactions
                .into_iter()
                .map(ImportedTransactionResponse::from)
                .collect();
            Ok(Json(ApiResponse {
                data: responses,
                message: "Imported transactions retrieved successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to retrieve imported transactions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get imported transactions for a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/imported-transactions",
    tag = "imported-transactions",
    params(
        ("account_id" = i32, Path, description = "Account ID")
    ),
    responses(
        (status = 200, description = "List of imported transactions for the account", body = ApiResponse<Vec<ImportedTransactionResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account_imported_transactions(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ImportedTransactionResponse>>>, StatusCode> {
    trace!("Entering get_account_imported_transactions function");
    debug!("Getting imported transactions for account_id: {}", account_id);

    match imported_transaction::Entity::find()
        .filter(imported_transaction::Column::AccountId.eq(account_id))
        .all(&state.db)
        .await
    {
        Ok(imported_transactions) => {
            info!("Successfully retrieved {} imported transactions for account {}", 
                  imported_transactions.len(), account_id);
            let responses: Vec<ImportedTransactionResponse> = imported_transactions
                .into_iter()
                .map(ImportedTransactionResponse::from)
                .collect();
            Ok(Json(ApiResponse {
                data: responses,
                message: "Account imported transactions retrieved successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to retrieve imported transactions for account {}: {}", account_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific imported transaction by ID
#[utoipa::path(
    get,
    path = "/api/v1/imported-transactions/{transaction_id}",
    tag = "imported-transactions",
    params(
        ("transaction_id" = i32, Path, description = "Imported transaction ID")
    ),
    responses(
        (status = 200, description = "Imported transaction details", body = ApiResponse<ImportedTransactionResponse>),
        (status = 404, description = "Imported transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_imported_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ImportedTransactionResponse>>, StatusCode> {
    trace!("Entering get_imported_transaction function");
    debug!("Getting imported transaction with id: {}", transaction_id);

    match imported_transaction::Entity::find_by_id(transaction_id).one(&state.db).await {
        Ok(Some(imported_transaction)) => {
            info!("Successfully retrieved imported transaction with id: {}", transaction_id);
            let response = ImportedTransactionResponse::from(imported_transaction);
            Ok(Json(ApiResponse {
                data: response,
                message: "Imported transaction retrieved successfully".to_string(),
                success: true,
            }))
        }
        Ok(None) => {
            warn!("Imported transaction with id {} not found", transaction_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to retrieve imported transaction with id {}: {}", transaction_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update an imported transaction
#[utoipa::path(
    put,
    path = "/api/v1/imported-transactions/{transaction_id}",
    tag = "imported-transactions",
    params(
        ("transaction_id" = i32, Path, description = "Imported transaction ID")
    ),
    request_body = UpdateImportedTransactionRequest,
    responses(
        (status = 200, description = "Imported transaction updated successfully", body = ApiResponse<ImportedTransactionResponse>),
        (status = 404, description = "Imported transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_imported_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateImportedTransactionRequest>,
) -> Result<Json<ApiResponse<ImportedTransactionResponse>>, StatusCode> {
    trace!("Entering update_imported_transaction function");
    debug!("Updating imported transaction with id: {}", transaction_id);

    // First, find the existing imported transaction
    let existing_imported_transaction = match imported_transaction::Entity::find_by_id(transaction_id).one(&state.db).await {
        Ok(Some(imported_transaction)) => imported_transaction,
        Ok(None) => {
            warn!("Imported transaction with id {} not found for update", transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to find imported transaction with id {} for update: {}", transaction_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create an active model for updating
    let mut imported_transaction_update: imported_transaction::ActiveModel = existing_imported_transaction.into();

    // Update fields if provided
    if let Some(date) = request.date {
        imported_transaction_update.date = Set(date);
    }
    if let Some(description) = request.description {
        imported_transaction_update.description = Set(description);
    }
    if let Some(amount) = request.amount {
        imported_transaction_update.amount = Set(amount);
    }
    if let Some(raw_data) = request.raw_data {
        imported_transaction_update.raw_data = Set(Some(sea_orm::JsonValue::from(raw_data)));
    }

    trace!("Attempting to update imported transaction in database");
    match imported_transaction_update.update(&state.db).await {
        Ok(updated_imported_transaction) => {
            info!("Successfully updated imported transaction with id: {}", transaction_id);
            let response = ImportedTransactionResponse::from(updated_imported_transaction);
            Ok(Json(ApiResponse {
                data: response,
                message: "Imported transaction updated successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to update imported transaction with id {}: {}", transaction_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete an imported transaction
#[utoipa::path(
    delete,
    path = "/api/v1/imported-transactions/{transaction_id}",
    tag = "imported-transactions",
    params(
        ("transaction_id" = i32, Path, description = "Imported transaction ID")
    ),
    responses(
        (status = 200, description = "Imported transaction deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Imported transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_imported_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    trace!("Entering delete_imported_transaction function");
    debug!("Deleting imported transaction with id: {}", transaction_id);

    // First, check if the imported transaction exists
    let _existing_imported_transaction = match imported_transaction::Entity::find_by_id(transaction_id).one(&state.db).await {
        Ok(Some(imported_transaction)) => imported_transaction,
        Ok(None) => {
            warn!("Imported transaction with id {} not found for deletion", transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to find imported transaction with id {} for deletion: {}", transaction_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Delete the imported transaction
    trace!("Attempting to delete imported transaction from database");
    match imported_transaction::Entity::delete_by_id(transaction_id).exec(&state.db).await {
        Ok(_) => {
            info!("Successfully deleted imported transaction with id: {}", transaction_id);
            Ok(Json(ApiResponse {
                data: format!("Imported transaction with id {} deleted successfully", transaction_id),
                message: "Imported transaction deleted successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to delete imported transaction with id {}: {}", transaction_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Reconcile an imported transaction with a real transaction
#[utoipa::path(
    post,
    path = "/api/v1/imported-transactions/{transaction_id}/reconcile",
    tag = "imported-transactions",
    params(
        ("transaction_id" = i32, Path, description = "Imported transaction ID")
    ),
    request_body = ReconcileImportedTransactionRequest,
    responses(
        (status = 200, description = "Imported transaction reconciled successfully", body = ApiResponse<ImportedTransactionResponse>),
        (status = 404, description = "Imported transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn reconcile_imported_transaction(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<ReconcileImportedTransactionRequest>,
) -> Result<Json<ApiResponse<ImportedTransactionResponse>>, StatusCode> {
    trace!("Entering reconcile_imported_transaction function");
    debug!("Reconciling imported transaction {} with {} transaction {}", 
           transaction_id, request.transaction_type, request.transaction_id);

    // Find the existing imported transaction
    let existing_imported_transaction = match imported_transaction::Entity::find_by_id(transaction_id).one(&state.db).await {
        Ok(Some(imported_transaction)) => imported_transaction,
        Ok(None) => {
            warn!("Imported transaction with id {} not found for reconciliation", transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to find imported transaction with id {} for reconciliation: {}", transaction_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Convert string to enum
    let transaction_type = match request.transaction_type.as_str() {
        "OneOff" => imported_transaction::ReconciledTransactionEntityType::OneOff,
        "Recurring" => imported_transaction::ReconciledTransactionEntityType::Recurring,
        "RecurringIncome" => imported_transaction::ReconciledTransactionEntityType::RecurringIncome,
        "RecurringInstance" => imported_transaction::ReconciledTransactionEntityType::RecurringInstance,
        _ => {
            warn!("Invalid transaction type: {}", request.transaction_type);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Update the reconciliation fields
    let mut imported_transaction_update: imported_transaction::ActiveModel = existing_imported_transaction.into();
    imported_transaction_update.reconciled_transaction_type = Set(Some(transaction_type));
    imported_transaction_update.reconciled_transaction_id = Set(Some(request.transaction_id));

    trace!("Attempting to update reconciliation in database");
    match imported_transaction_update.update(&state.db).await {
        Ok(updated_imported_transaction) => {
            info!("Successfully reconciled imported transaction with id: {}", transaction_id);
            let response = ImportedTransactionResponse::from(updated_imported_transaction);
            Ok(Json(ApiResponse {
                data: response,
                message: "Imported transaction reconciled successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to reconcile imported transaction with id {}: {}", transaction_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Clear reconciliation for an imported transaction
#[utoipa::path(
    delete,
    path = "/api/v1/imported-transactions/{transaction_id}/reconcile",
    tag = "imported-transactions",
    params(
        ("transaction_id" = i32, Path, description = "Imported transaction ID")
    ),
    responses(
        (status = 200, description = "Reconciliation cleared successfully", body = ApiResponse<ImportedTransactionResponse>),
        (status = 404, description = "Imported transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn clear_imported_transaction_reconciliation(
    Path(transaction_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ImportedTransactionResponse>>, StatusCode> {
    trace!("Entering clear_imported_transaction_reconciliation function");
    debug!("Clearing reconciliation for imported transaction {}", transaction_id);

    // Find the existing imported transaction
    let existing_imported_transaction = match imported_transaction::Entity::find_by_id(transaction_id).one(&state.db).await {
        Ok(Some(imported_transaction)) => imported_transaction,
        Ok(None) => {
            warn!("Imported transaction with id {} not found for clearing reconciliation", transaction_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to find imported transaction with id {} for clearing reconciliation: {}", transaction_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Clear the reconciliation fields
    let mut imported_transaction_update: imported_transaction::ActiveModel = existing_imported_transaction.into();
    imported_transaction_update.reconciled_transaction_type = Set(None);
    imported_transaction_update.reconciled_transaction_id = Set(None);

    trace!("Attempting to clear reconciliation in database");
    match imported_transaction_update.update(&state.db).await {
        Ok(updated_imported_transaction) => {
            info!("Successfully cleared reconciliation for imported transaction with id: {}", transaction_id);
            let response = ImportedTransactionResponse::from(updated_imported_transaction);
            Ok(Json(ApiResponse {
                data: response,
                message: "Reconciliation cleared successfully".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to clear reconciliation for imported transaction with id {}: {}", transaction_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
