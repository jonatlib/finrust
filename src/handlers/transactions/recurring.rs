use crate::schemas::{ApiResponse, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use model::entities::{recurring_transaction, recurring_transaction_instance};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

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
