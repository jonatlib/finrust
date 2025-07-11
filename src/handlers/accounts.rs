use crate::schemas::{ApiResponse, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use model::entities::account;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

/// Request body for creating a new account
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    /// Account name
    pub name: String,
    /// Account description
    pub description: Option<String>,
    /// ISO 4217 currency code (e.g., "USD", "EUR")
    pub currency_code: String,
    /// Owner user ID
    pub owner_id: i32,
    /// Whether to include in statistics (default: true)
    pub include_in_statistics: Option<bool>,
    /// Ledger name for export
    pub ledger_name: Option<String>,
}

/// Request body for updating an account
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    /// Account name
    pub name: Option<String>,
    /// Account description
    pub description: Option<String>,
    /// ISO 4217 currency code (e.g., "USD", "EUR")
    pub currency_code: Option<String>,
    /// Whether to include in statistics
    pub include_in_statistics: Option<bool>,
    /// Ledger name for export
    pub ledger_name: Option<String>,
}

/// Account response model
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub currency_code: String,
    pub owner_id: i32,
    pub include_in_statistics: bool,
    pub ledger_name: Option<String>,
}

impl From<account::Model> for AccountResponse {
    fn from(model: account::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            currency_code: model.currency_code,
            owner_id: model.owner_id,
            include_in_statistics: model.include_in_statistics,
            ledger_name: model.ledger_name,
        }
    }
}

/// Create a new account
#[utoipa::path(
    post,
    path = "/api/v1/accounts",
    tag = "accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Account created successfully", body = ApiResponse<AccountResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_account(
    State(state): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AccountResponse>>), StatusCode> {
    let new_account = account::ActiveModel {
        name: Set(request.name),
        description: Set(request.description),
        currency_code: Set(request.currency_code),
        owner_id: Set(request.owner_id),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        ledger_name: Set(request.ledger_name),
        ..Default::default()
    };

    match new_account.insert(&state.db).await {
        Ok(account_model) => {
            let response = ApiResponse {
                data: AccountResponse::from(account_model),
                message: "Account created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get all accounts
#[utoipa::path(
    get,
    path = "/api/v1/accounts",
    tag = "accounts",
    responses(
        (status = 200, description = "Accounts retrieved successfully", body = ApiResponse<Vec<AccountResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_accounts(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AccountResponse>>>, StatusCode> {
    match account::Entity::find().all(&state.db).await {
        Ok(accounts) => {
            let account_responses: Vec<AccountResponse> = accounts
                .into_iter()
                .map(AccountResponse::from)
                .collect();

            let response = ApiResponse {
                data: account_responses,
                message: "Accounts retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get a specific account by ID
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account retrieved successfully", body = ApiResponse<AccountResponse>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_account(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AccountResponse>>, StatusCode> {
    match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account_model)) => {
            let response = ApiResponse {
                data: AccountResponse::from(account_model),
                message: "Account retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Update an account
#[utoipa::path(
    put,
    path = "/api/v1/accounts/{account_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Account updated successfully", body = ApiResponse<AccountResponse>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_account(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateAccountRequest>,
) -> Result<Json<ApiResponse<AccountResponse>>, StatusCode> {
    // First, find the existing account
    let existing_account = match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account)) => account,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Create active model for update
    let mut account_active: account::ActiveModel = existing_account.into();

    // Update only provided fields
    if let Some(name) = request.name {
        account_active.name = Set(name);
    }
    if let Some(description) = request.description {
        account_active.description = Set(Some(description));
    }
    if let Some(currency_code) = request.currency_code {
        account_active.currency_code = Set(currency_code);
    }
    if let Some(include_in_statistics) = request.include_in_statistics {
        account_active.include_in_statistics = Set(include_in_statistics);
    }
    if let Some(ledger_name) = request.ledger_name {
        account_active.ledger_name = Set(Some(ledger_name));
    }

    match account_active.update(&state.db).await {
        Ok(updated_account) => {
            let response = ApiResponse {
                data: AccountResponse::from(updated_account),
                message: "Account updated successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Delete an account
#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{account_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
    ),
    responses(
        (status = 200, description = "Account deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_account(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match account::Entity::delete_by_id(account_id).exec(&state.db).await {
        Ok(delete_result) => {
            if delete_result.rows_affected > 0 {
                let response = ApiResponse {
                    data: format!("Account {} deleted", account_id),
                    message: "Account deleted successfully".to_string(),
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
