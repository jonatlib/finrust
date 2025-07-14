use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::{manual_account_state, account};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter, ModelTrait};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

/// Request body for creating a new manual account state
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateManualAccountStateRequest {
    /// The date the balance is valid for (YYYY-MM-DD)
    pub date: NaiveDate,
    /// The amount in the account on the specified date
    pub amount: Decimal,
}

/// Request body for updating a manual account state
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateManualAccountStateRequest {
    /// The date the balance is valid for (YYYY-MM-DD)
    pub date: Option<NaiveDate>,
    /// The amount in the account on the specified date
    pub amount: Option<Decimal>,
}

/// Manual account state response model
#[derive(Debug, Serialize, ToSchema)]
pub struct ManualAccountStateResponse {
    pub id: i32,
    pub account_id: i32,
    pub date: NaiveDate,
    pub amount: Decimal,
}

impl From<manual_account_state::Model> for ManualAccountStateResponse {
    fn from(model: manual_account_state::Model) -> Self {
        Self {
            id: model.id,
            account_id: model.account_id,
            date: model.date,
            amount: model.amount,
        }
    }
}

/// Create a new manual account state
#[utoipa::path(
    post,
    path = "/api/v1/accounts/{account_id}/manual-states",
    tag = "manual-account-states",
    request_body = CreateManualAccountStateRequest,
    responses(
        (status = 201, description = "Manual account state created successfully", body = ApiResponse<ManualAccountStateResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_manual_account_state(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<CreateManualAccountStateRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ManualAccountStateResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_manual_account_state function");
    debug!("Creating manual account state for account_id: {}, date: {}, amount: {}", 
           account_id, request.date, request.amount);

    // Validate that the account exists
    trace!("Validating account_id: {}", account_id);
    match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(_account)) => {
            debug!("Account with ID {} found, proceeding with manual account state creation", account_id);
        }
        Ok(None) => {
            warn!("Attempted to create manual account state for non-existent account_id: {}", account_id);
            let error_response = ErrorResponse {
                error: format!("Account with id {} does not exist", account_id),
                code: "INVALID_ACCOUNT_ID".to_string(),
                success: false,
            };
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(db_error) => {
            error!("Database error while validating account_id {}: {}", account_id, db_error);
            let error_response = ErrorResponse {
                error: "Internal server error while validating account".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

    // Create the manual account state
    let new_manual_state = manual_account_state::ActiveModel {
        account_id: Set(account_id),
        date: Set(request.date),
        amount: Set(request.amount),
        ..Default::default()
    };

    trace!("Attempting to save manual account state to database");
    match new_manual_state.insert(&state.db).await {
        Ok(manual_state) => {
            info!("Manual account state created successfully with ID: {}", manual_state.id);
            let response = ApiResponse {
                data: ManualAccountStateResponse::from(manual_state),
                message: "Manual account state created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(db_error) => {
            error!("Failed to create manual account state: {}", db_error);
            let error_response = ErrorResponse {
                error: "Failed to create manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Get all manual account states for an account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/manual-states",
    tag = "manual-account-states",
    responses(
        (status = 200, description = "Manual account states retrieved successfully", body = ApiResponse<Vec<ManualAccountStateResponse>>),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_manual_account_states(
    Path(account_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ManualAccountStateResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_manual_account_states function");
    debug!("Retrieving manual account states for account_id: {}", account_id);

    // Validate that the account exists
    trace!("Validating account_id: {}", account_id);
    match account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(_account)) => {
            debug!("Account with ID {} found, proceeding with manual account states retrieval", account_id);
        }
        Ok(None) => {
            warn!("Attempted to get manual account states for non-existent account_id: {}", account_id);
            let error_response = ErrorResponse {
                error: format!("Account with id {} does not exist", account_id),
                code: "INVALID_ACCOUNT_ID".to_string(),
                success: false,
            };
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(db_error) => {
            error!("Database error while validating account_id {}: {}", account_id, db_error);
            let error_response = ErrorResponse {
                error: "Internal server error while validating account".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    }

    trace!("Querying manual account states from database");
    match manual_account_state::Entity::find()
        .filter(manual_account_state::Column::AccountId.eq(account_id))
        .all(&state.db)
        .await
    {
        Ok(manual_states) => {
            debug!("Retrieved {} manual account states for account_id: {}", manual_states.len(), account_id);
            let response_data: Vec<ManualAccountStateResponse> = manual_states
                .into_iter()
                .map(ManualAccountStateResponse::from)
                .collect();

            let response = ApiResponse {
                data: response_data,
                message: "Manual account states retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to retrieve manual account states for account_id {}: {}", account_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to retrieve manual account states".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Get a specific manual account state by ID
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}/manual-states/{state_id}",
    tag = "manual-account-states",
    responses(
        (status = 200, description = "Manual account state retrieved successfully", body = ApiResponse<ManualAccountStateResponse>),
        (status = 404, description = "Manual account state not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_manual_account_state(
    Path((account_id, state_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ManualAccountStateResponse>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_manual_account_state function");
    debug!("Retrieving manual account state with ID: {} for account_id: {}", state_id, account_id);

    trace!("Querying manual account state from database");
    match manual_account_state::Entity::find()
        .filter(manual_account_state::Column::Id.eq(state_id))
        .filter(manual_account_state::Column::AccountId.eq(account_id))
        .one(&state.db)
        .await
    {
        Ok(Some(manual_state)) => {
            debug!("Manual account state with ID {} found", state_id);
            let response = ApiResponse {
                data: ManualAccountStateResponse::from(manual_state),
                message: "Manual account state retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Ok(None) => {
            warn!("Manual account state with ID {} not found for account_id: {}", state_id, account_id);
            let error_response = ErrorResponse {
                error: format!("Manual account state with id {} not found for account {}", state_id, account_id),
                code: "NOT_FOUND".to_string(),
                success: false,
            };
            Err((StatusCode::NOT_FOUND, Json(error_response)))
        }
        Err(db_error) => {
            error!("Failed to retrieve manual account state with ID {}: {}", state_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to retrieve manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Update a manual account state
#[utoipa::path(
    put,
    path = "/api/v1/accounts/{account_id}/manual-states/{state_id}",
    tag = "manual-account-states",
    request_body = UpdateManualAccountStateRequest,
    responses(
        (status = 200, description = "Manual account state updated successfully", body = ApiResponse<ManualAccountStateResponse>),
        (status = 404, description = "Manual account state not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_manual_account_state(
    Path((account_id, state_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    Json(request): Json<UpdateManualAccountStateRequest>,
) -> Result<Json<ApiResponse<ManualAccountStateResponse>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering update_manual_account_state function");
    debug!("Updating manual account state with ID: {} for account_id: {}", state_id, account_id);

    // Find the existing manual account state
    trace!("Querying existing manual account state from database");
    let existing_state = match manual_account_state::Entity::find()
        .filter(manual_account_state::Column::Id.eq(state_id))
        .filter(manual_account_state::Column::AccountId.eq(account_id))
        .one(&state.db)
        .await
    {
        Ok(Some(state)) => state,
        Ok(None) => {
            warn!("Manual account state with ID {} not found for account_id: {}", state_id, account_id);
            let error_response = ErrorResponse {
                error: format!("Manual account state with id {} not found for account {}", state_id, account_id),
                code: "NOT_FOUND".to_string(),
                success: false,
            };
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(db_error) => {
            error!("Failed to retrieve manual account state with ID {}: {}", state_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to retrieve manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    };

    // Create the updated model
    let mut updated_state: manual_account_state::ActiveModel = existing_state.into();

    if let Some(date) = request.date {
        updated_state.date = Set(date);
    }
    if let Some(amount) = request.amount {
        updated_state.amount = Set(amount);
    }

    trace!("Attempting to update manual account state in database");
    match updated_state.update(&state.db).await {
        Ok(updated_model) => {
            info!("Manual account state with ID {} updated successfully", state_id);
            let response = ApiResponse {
                data: ManualAccountStateResponse::from(updated_model),
                message: "Manual account state updated successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to update manual account state with ID {}: {}", state_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to update manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Delete a manual account state
#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{account_id}/manual-states/{state_id}",
    tag = "manual-account-states",
    responses(
        (status = 200, description = "Manual account state deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Manual account state not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_manual_account_state(
    Path((account_id, state_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering delete_manual_account_state function");
    debug!("Deleting manual account state with ID: {} for account_id: {}", state_id, account_id);

    // Find the existing manual account state
    trace!("Querying existing manual account state from database");
    let existing_state = match manual_account_state::Entity::find()
        .filter(manual_account_state::Column::Id.eq(state_id))
        .filter(manual_account_state::Column::AccountId.eq(account_id))
        .one(&state.db)
        .await
    {
        Ok(Some(state)) => state,
        Ok(None) => {
            warn!("Manual account state with ID {} not found for account_id: {}", state_id, account_id);
            let error_response = ErrorResponse {
                error: format!("Manual account state with id {} not found for account {}", state_id, account_id),
                code: "NOT_FOUND".to_string(),
                success: false,
            };
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(db_error) => {
            error!("Failed to retrieve manual account state with ID {}: {}", state_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to retrieve manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    };

    // Delete the manual account state
    trace!("Attempting to delete manual account state from database");
    match existing_state.delete(&state.db).await {
        Ok(_) => {
            info!("Manual account state with ID {} deleted successfully", state_id);
            let response = ApiResponse {
                data: format!("Manual account state with id {} deleted successfully", state_id),
                message: "Manual account state deleted successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to delete manual account state with ID {}: {}", state_id, db_error);
            let error_response = ErrorResponse {
                error: "Failed to delete manual account state".to_string(),
                code: "DATABASE_ERROR".to_string(),
                success: false,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}
