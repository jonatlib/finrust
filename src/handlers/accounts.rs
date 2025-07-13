use crate::schemas::{ApiResponse, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use model::entities::account;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

/// Request body for creating a new account
#[derive(Debug, Deserialize, Serialize, ToSchema)]
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
#[derive(Debug, Deserialize, Serialize, ToSchema)]
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
    trace!("Entering create_account function");
    debug!("Creating account with name: {}, currency: {}, owner_id: {}", 
           request.name, request.currency_code, request.owner_id);

    let new_account = account::ActiveModel {
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        currency_code: Set(request.currency_code.clone()),
        owner_id: Set(request.owner_id),
        include_in_statistics: Set(request.include_in_statistics.unwrap_or(true)),
        ledger_name: Set(request.ledger_name.clone()),
        ..Default::default()
    };

    trace!("Attempting to insert new account into database");
    match new_account.insert(&state.db).await {
        Ok(account_model) => {
            info!("Account created successfully with ID: {}, name: {}", 
                  account_model.id, account_model.name);
            let response = ApiResponse {
                data: AccountResponse::from(account_model),
                message: "Account created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(db_error) => {
            error!("Failed to create account '{}' for owner {}: {}", 
                   request.name, request.owner_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    trace!("Entering get_accounts function");
    debug!("Fetching all accounts from database");

    match account::Entity::find().all(&state.db).await {
        Ok(accounts) => {
            let account_count = accounts.len();
            debug!("Retrieved {} accounts from database", account_count);

            let account_responses: Vec<AccountResponse> = accounts
                .into_iter()
                .map(AccountResponse::from)
                .collect();

            info!("Successfully retrieved {} accounts", account_count);
            let response = ApiResponse {
                data: account_responses,
                message: "Accounts retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to retrieve accounts from database: {}", db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    trace!("Entering get_account function for account_id: {}", account_id);
    debug!("Fetching account with ID: {}", account_id);

    match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account_model)) => {
            info!("Successfully retrieved account with ID: {}, name: {}", 
                  account_model.id, account_model.name);
            let response = ApiResponse {
                data: AccountResponse::from(account_model),
                message: "Account retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Ok(None) => {
            warn!("Account with ID {} not found", account_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(db_error) => {
            error!("Failed to retrieve account with ID {}: {}", account_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    trace!("Entering update_account function for account_id: {}", account_id);
    debug!("Updating account with ID: {}", account_id);

    // First, find the existing account
    trace!("Looking up existing account with ID: {}", account_id);
    let existing_account = match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(account)) => {
            debug!("Found existing account: {}", account.name);
            account
        }
        Ok(None) => {
            warn!("Account with ID {} not found for update", account_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(db_error) => {
            error!("Failed to lookup account with ID {} for update: {}", account_id, db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create active model for update
    let mut account_active: account::ActiveModel = existing_account.into();
    let mut updated_fields = Vec::new();

    // Update only provided fields
    if let Some(name) = request.name {
        debug!("Updating account name to: {}", name);
        account_active.name = Set(name.clone());
        updated_fields.push(format!("name: {}", name));
    }
    if let Some(description) = request.description {
        debug!("Updating account description");
        account_active.description = Set(Some(description.clone()));
        updated_fields.push(format!("description: {:?}", description));
    }
    if let Some(currency_code) = request.currency_code {
        debug!("Updating account currency_code to: {}", currency_code);
        account_active.currency_code = Set(currency_code.clone());
        updated_fields.push(format!("currency_code: {}", currency_code));
    }
    if let Some(include_in_statistics) = request.include_in_statistics {
        debug!("Updating account include_in_statistics to: {}", include_in_statistics);
        account_active.include_in_statistics = Set(include_in_statistics);
        updated_fields.push(format!("include_in_statistics: {}", include_in_statistics));
    }
    if let Some(ledger_name) = request.ledger_name {
        debug!("Updating account ledger_name to: {:?}", ledger_name);
        account_active.ledger_name = Set(Some(ledger_name.clone()));
        updated_fields.push(format!("ledger_name: {:?}", ledger_name));
    }

    if updated_fields.is_empty() {
        debug!("No fields to update for account ID: {}", account_id);
    } else {
        debug!("Updating fields: {}", updated_fields.join(", "));
    }

    trace!("Attempting to update account in database");
    match account_active.update(&state.db).await {
        Ok(updated_account) => {
            info!("Account with ID {} updated successfully. Updated fields: {}", 
                  account_id, if updated_fields.is_empty() { "none".to_string() } else { updated_fields.join(", ") });
            let response = ApiResponse {
                data: AccountResponse::from(updated_account),
                message: "Account updated successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to update account with ID {}: {}", account_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    trace!("Entering delete_account function for account_id: {}", account_id);
    debug!("Attempting to delete account with ID: {}", account_id);

    match account::Entity::delete_by_id(account_id).exec(&state.db).await {
        Ok(delete_result) => {
            debug!("Delete operation completed. Rows affected: {}", delete_result.rows_affected);
            if delete_result.rows_affected > 0 {
                info!("Account with ID {} deleted successfully", account_id);
                let response = ApiResponse {
                    data: format!("Account {} deleted", account_id),
                    message: "Account deleted successfully".to_string(),
                    success: true,
                };
                Ok(Json(response))
            } else {
                warn!("Account with ID {} not found for deletion (no rows affected)", account_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(db_error) => {
            error!("Failed to delete account with ID {}: {}", account_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

