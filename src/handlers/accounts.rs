use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use model::entities::{account, user, tag, account_tag, account_allowed_user};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, DbErr, ColumnTrait, QueryFilter, PaginatorTrait};
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
) -> Result<(StatusCode, Json<ApiResponse<AccountResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_account function");
    debug!("Creating account with name: {}, currency: {}, owner_id: {}", 
           request.name, request.currency_code, request.owner_id);

    // Validate that the owner exists
    trace!("Validating owner_id: {}", request.owner_id);
    match user::Entity::find_by_id(request.owner_id).one(&state.db).await {
        Ok(Some(_user)) => {
            debug!("Owner with ID {} found, proceeding with account creation", request.owner_id);
        }
        Ok(None) => {
            warn!("Attempted to create account with non-existent owner_id: {}", request.owner_id);
            let error_response = ErrorResponse {
                error: format!("Owner with id {} does not exist", request.owner_id),
                code: "INVALID_OWNER_ID".to_string(),
                success: false,
            };
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
        Err(db_error) => {
            error!("Database error while validating owner_id {}: {}", request.owner_id, db_error);
            let error_response = ErrorResponse {
                error: "Internal server error while validating owner".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

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

            // Handle specific database errors
            let error_response = match db_error {
                DbErr::Exec(ref exec_err) => {
                    // Check for foreign key constraint violations or other specific errors
                    let error_msg = exec_err.to_string().to_lowercase();
                    if error_msg.contains("foreign key") || error_msg.contains("constraint") {
                        ErrorResponse {
                            error: format!("Invalid owner_id: {}", request.owner_id),
                            code: "FOREIGN_KEY_VIOLATION".to_string(),
                            success: false,
                        }
                    } else {
                        ErrorResponse {
                            error: "Failed to create account due to database constraint".to_string(),
                            code: "DATABASE_CONSTRAINT_ERROR".to_string(),
                            success: false,
                        }
                    }
                }
                _ => ErrorResponse {
                    error: "Internal server error while creating account".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }
            };

            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
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


// ----- Account ↔ Tag and Allowed User relations -----
use crate::handlers::tags::TagResponse as TagDto;
use crate::handlers::users::UserResponse as UserDto;
use sea_orm::{DeleteResult};

/// Response when linking a tag to an account
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountTagLinkResponse {
    pub account_id: i32,
    pub tag_id: i32,
}

/// Response when linking an allowed user to an account
#[derive(Debug, Serialize, ToSchema)]
pub struct AllowedUserLinkResponse {
    pub account_id: i32,
    pub user_id: i32,
}

#[utoipa::path(
    put,
    path = "/api/v1/accounts/{account_id}/tags/{tag_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
        ("tag_id" = i32, Path, description = "Tag ID"),
    ),
    responses(
        (status = 200, description = "Tag linked to account", body = ApiResponse<AccountTagLinkResponse>),
        (status = 404, description = "Account or Tag not found", body = ErrorResponse),
        (status = 409, description = "Link already exists", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn link_account_tag(
    State(state): State<AppState>,
    Path((account_id, tag_id)): Path<(i32, i32)>,
) -> Result<(StatusCode, Json<ApiResponse<AccountTagLinkResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!(account_id, tag_id, "link_account_tag");
    // Validate existence
    if account::Entity::find_by_id(account_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error while checking account");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        warn!(account_id, "Account not found");
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: format!("Account {} not found", account_id), code: "NOT_FOUND".into(), success: false })));
    }
    if tag::Entity::find_by_id(tag_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error while checking tag");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        warn!(tag_id, "Tag not found");
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: format!("Tag {} not found", tag_id), code: "NOT_FOUND".into(), success: false })));
    }

    let am = account_tag::ActiveModel { account_id: Set(account_id), tag_id: Set(tag_id) };
    match am.insert(&state.db).await {
        Ok(_) => {
            info!(account_id, tag_id, "Linked tag to account");
            Ok((StatusCode::OK, Json(ApiResponse{ data: AccountTagLinkResponse{ account_id, tag_id }, message: "Linked".into(), success: true })))
        }
        Err(DbErr::Exec(e)) => {
            let msg = e.to_string();
            if msg.to_lowercase().contains("unique") { 
                warn!("Link already exists");
                Err((StatusCode::CONFLICT, Json(ErrorResponse{ error: "Link already exists".into(), code: "CONFLICT".into(), success: false })))
            } else if msg.to_lowercase().contains("foreign key") {
                Err((StatusCode::BAD_REQUEST, Json(ErrorResponse{ error: "Foreign key violation".into(), code: "FOREIGN_KEY_VIOLATION".into(), success: false })))
            } else {
                error!(%msg, "Database constraint error");
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false })))
            }
        }
        Err(e) => {
            error!(%e, "Database error inserting link");
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false })))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{account_id}/tags/{tag_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
        ("tag_id" = i32, Path, description = "Tag ID"),
    ),
    responses(
        (status = 200, description = "Tag unlinked from account", body = ApiResponse<AccountTagLinkResponse>),
        (status = 404, description = "Account or Tag not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn unlink_account_tag(
    State(state): State<AppState>,
    Path((account_id, tag_id)): Path<(i32, i32)>,
) -> Result<(StatusCode, Json<ApiResponse<AccountTagLinkResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!(account_id, tag_id, "unlink_account_tag");
    // Validate account exists
    let account_exists = account::Entity::find_by_id(account_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB err");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;
    if account_exists.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Account not found".into(), code: "NOT_FOUND".into(), success: false })));
    }
    // Validate tag exists
    let tag_exists = tag::Entity::find_by_id(tag_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB err");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;
    if tag_exists.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Tag not found".into(), code: "NOT_FOUND".into(), success: false })));
    }

    let res: DeleteResult = account_tag::Entity::delete_many()
        .filter(account_tag::Column::AccountId.eq(account_id))
        .filter(account_tag::Column::TagId.eq(tag_id))
        .exec(&state.db).await.map_err(|e| {
            error!(%e, "DB error deleting link");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;
    debug!(rows = res.rows_affected, "Unlink rows affected");
    Ok((StatusCode::OK, Json(ApiResponse{ data: AccountTagLinkResponse{ account_id, tag_id }, message: "Unlinked".into(), success: true })))
}

#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/tags",
    tag = "accounts",
    params(("account_id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "List of tags for account", body = ApiResponse<Vec<crate::handlers::tags::TagResponse>>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn get_account_tags(
    State(state): State<AppState>,
    Path(account_id): Path<i32>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<TagDto>>>), (StatusCode, Json<ErrorResponse>)> {
    // Ensure account exists
    if account::Entity::find_by_id(account_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error while checking account");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Account not found".into(), code: "NOT_FOUND".into(), success: false })));
    }

    // Fetch link rows, then load tags by IDs to avoid requiring Related<Tag> impl on link entity
    let tag_ids: Vec<i32> = account_tag::Entity::find()
        .filter(account_tag::Column::AccountId.eq(account_id))
        .all(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB error listing account tag links");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?
        .into_iter()
        .map(|link| link.tag_id)
        .collect();

    if tag_ids.is_empty() {
        return Ok((StatusCode::OK, Json(ApiResponse{ data: Vec::new(), message: "Success".into(), success: true })));
    }

    let tag_models = tag::Entity::find()
        .filter(tag::Column::Id.is_in(tag_ids))
        .all(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB error loading tags by IDs");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;

    let tags: Vec<TagDto> = tag_models.into_iter().map(TagDto::from).collect();
    Ok((StatusCode::OK, Json(ApiResponse{ data: tags, message: "Success".into(), success: true })))
}

#[utoipa::path(
    put,
    path = "/api/v1/accounts/{account_id}/allowed-users/{user_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
        ("user_id" = i32, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User granted access", body = ApiResponse<AllowedUserLinkResponse>),
        (status = 404, description = "Account or User not found", body = ErrorResponse),
        (status = 409, description = "Access already granted", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn link_account_allowed_user(
    State(state): State<AppState>,
    Path((account_id, user_id)): Path<(i32, i32)>,
) -> Result<(StatusCode, Json<ApiResponse<AllowedUserLinkResponse>>), (StatusCode, Json<ErrorResponse>)> {
    // Validate existence
    if account::Entity::find_by_id(account_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Account not found".into(), code: "NOT_FOUND".into(), success: false })));
    }
    if user::Entity::find_by_id(user_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "User not found".into(), code: "NOT_FOUND".into(), success: false })));
    }

    let am = account_allowed_user::ActiveModel { account_id: Set(account_id), user_id: Set(user_id) };
    match am.insert(&state.db).await {
        Ok(_) => Ok((StatusCode::OK, Json(ApiResponse{ data: AllowedUserLinkResponse{ account_id, user_id }, message: "Linked".into(), success: true }))),
        Err(DbErr::Exec(e)) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique") { Err((StatusCode::CONFLICT, Json(ErrorResponse{ error: "Already granted".into(), code: "CONFLICT".into(), success: false }))) }
            else if msg.contains("foreign key") { Err((StatusCode::BAD_REQUEST, Json(ErrorResponse{ error: "Foreign key violation".into(), code: "FOREIGN_KEY_VIOLATION".into(), success: false }))) }
            else { Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))) }
        }
        Err(e) => { error!(%e, "DB error"); Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))) }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{account_id}/allowed-users/{user_id}",
    tag = "accounts",
    params(
        ("account_id" = i32, Path, description = "Account ID"),
        ("user_id" = i32, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User access revoked", body = ApiResponse<AllowedUserLinkResponse>),
        (status = 404, description = "Account or User not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn unlink_account_allowed_user(
    State(state): State<AppState>,
    Path((account_id, user_id)): Path<(i32, i32)>,
) -> Result<(StatusCode, Json<ApiResponse<AllowedUserLinkResponse>>), (StatusCode, Json<ErrorResponse>)> {
    // Validate account & user exist
    if account::Entity::find_by_id(account_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() { return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Account not found".into(), code: "NOT_FOUND".into(), success: false }))); }
    if user::Entity::find_by_id(user_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() { return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "User not found".into(), code: "NOT_FOUND".into(), success: false }))); }

    let res = account_allowed_user::Entity::delete_many()
        .filter(account_allowed_user::Column::AccountId.eq(account_id))
        .filter(account_allowed_user::Column::UserId.eq(user_id))
        .exec(&state.db).await.map_err(|e| {
            error!(%e, "DB error deleting allowed user link");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;
    debug!(rows = res.rows_affected, "Revoked access");
    Ok((StatusCode::OK, Json(ApiResponse{ data: AllowedUserLinkResponse{ account_id, user_id }, message: "Revoked".into(), success: true })))
}

#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/allowed-users",
    tag = "accounts",
    params(("account_id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "List allowed users", body = ApiResponse<Vec<crate::handlers::users::UserResponse>>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
#[instrument]
pub async fn get_account_allowed_users(
    State(state): State<AppState>,
    Path(account_id): Path<i32>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<UserDto>>>), (StatusCode, Json<ErrorResponse>)> {
    if account::Entity::find_by_id(account_id).one(&state.db).await.map_err(|e| {
        error!(%e, "DB error while checking account");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
    })?.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse{ error: "Account not found".into(), code: "NOT_FOUND".into(), success: false })));
    }

    // Fetch link rows, then load users by IDs to avoid requiring Related<User> impl on link entity
    let user_ids: Vec<i32> = account_allowed_user::Entity::find()
        .filter(account_allowed_user::Column::AccountId.eq(account_id))
        .all(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB error listing allowed user links");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?
        .into_iter()
        .map(|link| link.user_id)
        .collect();

    if user_ids.is_empty() {
        return Ok((StatusCode::OK, Json(ApiResponse{ data: Vec::new(), message: "Success".into(), success: true })));
    }

    let user_models = user::Entity::find()
        .filter(user::Column::Id.is_in(user_ids))
        .all(&state.db)
        .await
        .map_err(|e| {
            error!(%e, "DB error loading users by IDs");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse{ error: "Database error".into(), code: "DATABASE_ERROR".into(), success: false }))
        })?;

    let users: Vec<UserDto> = user_models.into_iter().map(UserDto::from).collect();
    Ok((StatusCode::OK, Json(ApiResponse{ data: users, message: "Success".into(), success: true })))
}
