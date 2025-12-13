use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use chrono::NaiveDate;
use model::entities::{recurring_transaction, recurring_transaction_instance};
use model::transaction::TransactionGenerator;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, trace, warn};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use super::recurring::{RecurringInstanceResponse, TagInfo};

/// Query parameters for listing recurring transaction instances
#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct RecurringInstanceQuery {
    /// Page number (default: 1)
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    /// Page size (default: 50)
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<u64>,
    /// Filter by recurring transaction ID
    pub recurring_transaction_id: Option<i32>,
    /// Filter by status
    pub status: Option<String>,
}

/// Request body for updating a recurring transaction instance
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateRecurringInstanceRequest {
    /// Update the status
    pub status: Option<String>,
    /// Update the due date
    pub due_date: Option<NaiveDate>,
    /// Update the expected amount
    pub expected_amount: Option<Decimal>,
    /// Set the paid date
    pub paid_date: Option<NaiveDate>,
    /// Set the paid amount
    pub paid_amount: Option<Decimal>,
}

/// Get all recurring transaction instances
#[utoipa::path(
    get,
    path = "/api/v1/recurring-instances",
    tag = "recurring-transactions",
    params(RecurringInstanceQuery),
    responses(
        (status = 200, description = "Recurring transaction instances retrieved successfully", body = ApiResponse<Vec<RecurringInstanceResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_instances(
    Valid(Query(query)): Valid<Query<RecurringInstanceQuery>>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<RecurringInstanceResponse>>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_instances function");

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(100);

    debug!("Fetching recurring instances - page: {}, limit: {}", page, limit);

    let mut query_builder = recurring_transaction_instance::Entity::find();

    // Apply filters
    if let Some(recurring_id) = query.recurring_transaction_id {
        query_builder = query_builder.filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(recurring_id));
    }

    if let Some(status_str) = query.status {
        if let Ok(status) = parse_instance_status(&status_str) {
            query_builder = query_builder.filter(recurring_transaction_instance::Column::Status.eq(status));
        }
    }

    match query_builder
        .order_by_desc(recurring_transaction_instance::Column::DueDate)
        .paginate(&state.db, limit)
        .fetch_page(page - 1)
        .await
    {
        Ok(instances) => {
            info!("Successfully retrieved {} recurring instances", instances.len());

            let mut response_data = Vec::new();
            for instance in instances {
                match RecurringInstanceResponse::with_tags(instance.clone(), &state.db).await {
                    Ok(response) => response_data.push(response),
                    Err(tag_error) => {
                        warn!("Failed to fetch tags for recurring instance {}: {}", instance.id, tag_error);
                        response_data.push(RecurringInstanceResponse::from(instance));
                    }
                }
            }

            let response = ApiResponse {
                data: response_data,
                message: "Recurring instances retrieved successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            error!("Failed to retrieve recurring instances: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring instances".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Get a specific recurring transaction instance by ID
#[utoipa::path(
    get,
    path = "/api/v1/recurring-instances/{instance_id}",
    tag = "recurring-transactions",
    responses(
        (status = 200, description = "Recurring instance retrieved successfully", body = ApiResponse<RecurringInstanceResponse>),
        (status = 404, description = "Recurring instance not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_recurring_instance(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringInstanceResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_recurring_instance function");
    debug!("Fetching recurring instance with ID: {}", instance_id);

    match recurring_transaction_instance::Entity::find_by_id(instance_id)
        .one(&state.db)
        .await
    {
        Ok(Some(instance)) => {
            info!("Successfully retrieved recurring instance ID: {}", instance.id);

            match RecurringInstanceResponse::with_tags(instance.clone(), &state.db).await {
                Ok(instance_response) => {
                    let response = ApiResponse {
                        data: instance_response,
                        message: "Recurring instance retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for recurring instance {}: {}", instance.id, tag_error);
                    let response = ApiResponse {
                        data: RecurringInstanceResponse::from(instance),
                        message: "Recurring instance retrieved successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
            }
        }
        Ok(None) => {
            warn!("Recurring instance with ID {} not found", instance_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring instance with id {} does not exist", instance_id),
                    code: "INSTANCE_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while fetching recurring instance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring instance".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Update a recurring transaction instance
#[utoipa::path(
    put,
    path = "/api/v1/recurring-instances/{instance_id}",
    tag = "recurring-transactions",
    request_body = UpdateRecurringInstanceRequest,
    responses(
        (status = 200, description = "Recurring instance updated successfully", body = ApiResponse<RecurringInstanceResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Recurring instance not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_recurring_instance(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateRecurringInstanceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RecurringInstanceResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering update_recurring_instance function");
    debug!("Updating recurring instance with ID: {}", instance_id);

    // First, fetch the existing instance
    let existing_instance = match recurring_transaction_instance::Entity::find_by_id(instance_id)
        .one(&state.db)
        .await
    {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            warn!("Recurring instance with ID {} not found", instance_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring instance with id {} does not exist", instance_id),
                    code: "INSTANCE_NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Database error while fetching recurring instance: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve recurring instance".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Parse status if provided
    let status = if let Some(status_str) = &request.status {
        match parse_instance_status(status_str) {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Invalid instance status: {}", e);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e,
                        code: "INVALID_STATUS".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    } else {
        None
    };

    // Create the update model
    let mut update_model: recurring_transaction_instance::ActiveModel = existing_instance.into();

    if let Some(s) = status {
        update_model.status = Set(s);
    }
    if let Some(due_date) = request.due_date {
        update_model.due_date = Set(due_date);
    }
    if let Some(expected_amount) = request.expected_amount {
        update_model.expected_amount = Set(expected_amount);
    }
    if let Some(paid_date) = request.paid_date {
        update_model.paid_date = Set(Some(paid_date));
    }
    if let Some(paid_amount) = request.paid_amount {
        update_model.paid_amount = Set(Some(paid_amount));
    }

    match update_model.update(&state.db).await {
        Ok(updated_instance) => {
            info!("Successfully updated recurring instance with ID: {}", updated_instance.id);

            match RecurringInstanceResponse::with_tags(updated_instance.clone(), &state.db).await {
                Ok(instance_response) => {
                    let response = ApiResponse {
                        data: instance_response,
                        message: "Recurring instance updated successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(tag_error) => {
                    warn!("Failed to fetch tags for updated recurring instance {}: {}", updated_instance.id, tag_error);
                    let response = ApiResponse {
                        data: RecurringInstanceResponse::from(updated_instance),
                        message: "Recurring instance updated successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
            }
        }
        Err(e) => {
            error!("Failed to update recurring instance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update recurring instance".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

/// Delete a recurring transaction instance
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-instances/{instance_id}",
    tag = "recurring-transactions",
    responses(
        (status = 200, description = "Recurring instance deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Recurring instance not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_recurring_instance(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering delete_recurring_instance function");
    debug!("Deleting recurring instance with ID: {}", instance_id);

    // First, check if the instance exists
    match recurring_transaction_instance::Entity::find_by_id(instance_id)
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            // Instance exists, proceed with deletion
            match recurring_transaction_instance::Entity::delete_by_id(instance_id)
                .exec(&state.db)
                .await
            {
                Ok(_) => {
                    info!("Successfully deleted recurring instance with ID: {}", instance_id);
                    let response = ApiResponse {
                        data: format!("Recurring instance with id {} deleted successfully", instance_id),
                        message: "Recurring instance deleted successfully".to_string(),
                        success: true,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(e) => {
                    error!("Failed to delete recurring instance: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to delete recurring instance".to_string(),
                            code: "DATABASE_ERROR".to_string(),
                            success: false,
                        }),
                    ))
                }
            }
        }
        Ok(None) => {
            warn!("Recurring instance with ID {} not found", instance_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Recurring instance with id {} does not exist", instance_id),
                    code: "INSTANCE_NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Database error while checking recurring instance existence: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check recurring instance existence".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }),
            ))
        }
    }
}

// Helper function to parse instance status string to InstanceStatus enum
fn parse_instance_status(status_str: &str) -> Result<recurring_transaction_instance::InstanceStatus, String> {
    match status_str {
        "Pending" => Ok(recurring_transaction_instance::InstanceStatus::Pending),
        "Paid" => Ok(recurring_transaction_instance::InstanceStatus::Paid),
        "Skipped" => Ok(recurring_transaction_instance::InstanceStatus::Skipped),
        _ => Err(format!("Invalid instance status: {}", status_str)),
    }
}
