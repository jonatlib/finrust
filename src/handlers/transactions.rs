use crate::schemas::{ApiResponse, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::one_off_transaction;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

/// Request body for creating a new one-off transaction
#[derive(Debug, Deserialize, ToSchema)]
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
        }
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
) -> Result<(StatusCode, Json<ApiResponse<TransactionResponse>>), StatusCode> {
    let new_transaction = one_off_transaction::ActiveModel {
        name: Set(request.name),
        description: Set(request.description),
        amount: Set(request.amount),
        date: Set(request.date),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        target_account_id: Set(request.target_account_id),
        source_account_id: Set(request.source_account_id),
        ledger_name: Set(request.ledger_name),
        linked_import_id: Set(request.linked_import_id),
        ..Default::default()
    };

    match new_transaction.insert(&state.db).await {
        Ok(transaction_model) => {
            let response = ApiResponse {
                data: TransactionResponse::from(transaction_model),
                message: "Transaction created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
    match one_off_transaction::Entity::find().all(&state.db).await {
        Ok(transactions) => {
            let transaction_responses: Vec<TransactionResponse> = transactions
                .into_iter()
                .map(TransactionResponse::from)
                .collect();

            let response = ApiResponse {
                data: transaction_responses,
                message: "Transactions retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
    use sea_orm::{ColumnTrait, Condition, QueryFilter};

    // Find transactions where the account is either target or source
    let condition = Condition::any()
        .add(one_off_transaction::Column::TargetAccountId.eq(account_id))
        .add(one_off_transaction::Column::SourceAccountId.eq(account_id));

    match one_off_transaction::Entity::find()
        .filter(condition)
        .all(&state.db)
        .await
    {
        Ok(transactions) => {
            let transaction_responses: Vec<TransactionResponse> = transactions
                .into_iter()
                .map(TransactionResponse::from)
                .collect();

            let response = ApiResponse {
                data: transaction_responses,
                message: "Account transactions retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
    match one_off_transaction::Entity::find_by_id(transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction_model)) => {
            let response = ApiResponse {
                data: TransactionResponse::from(transaction_model),
                message: "Transaction retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
    // First, find the existing transaction
    let existing_transaction = match one_off_transaction::Entity::find_by_id(transaction_id)
        .one(&state.db)
        .await
    {
        Ok(Some(transaction)) => transaction,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Create active model for update
    let mut transaction_active: one_off_transaction::ActiveModel = existing_transaction.into();

    // Update only provided fields
    if let Some(name) = request.name {
        transaction_active.name = Set(name);
    }
    if let Some(description) = request.description {
        transaction_active.description = Set(Some(description));
    }
    if let Some(amount) = request.amount {
        transaction_active.amount = Set(amount);
    }
    if let Some(date) = request.date {
        transaction_active.date = Set(date);
    }
    if let Some(include_in_statistics) = request.include_in_statistics {
        transaction_active.include_in_statistics = Set(include_in_statistics);
    }
    if let Some(target_account_id) = request.target_account_id {
        transaction_active.target_account_id = Set(target_account_id);
    }
    if let Some(source_account_id) = request.source_account_id {
        transaction_active.source_account_id = Set(Some(source_account_id));
    }
    if let Some(ledger_name) = request.ledger_name {
        transaction_active.ledger_name = Set(Some(ledger_name));
    }
    if let Some(linked_import_id) = request.linked_import_id {
        transaction_active.linked_import_id = Set(Some(linked_import_id));
    }

    match transaction_active.update(&state.db).await {
        Ok(updated_transaction) => {
            let response = ApiResponse {
                data: TransactionResponse::from(updated_transaction),
                message: "Transaction updated successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
    match one_off_transaction::Entity::delete_by_id(transaction_id)
        .exec(&state.db)
        .await
    {
        Ok(delete_result) => {
            if delete_result.rows_affected > 0 {
                let response = ApiResponse {
                    data: format!("Transaction {} deleted", transaction_id),
                    message: "Transaction deleted successfully".to_string(),
                    success: true,
                };
                Ok(Json(response))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
