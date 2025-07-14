use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use model::entities::user;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, DbErr};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

/// Request body for creating a new user
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateUserRequest {
    /// Username (must be unique)
    pub username: String,
}

/// Request body for updating a user
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserRequest {
    /// Username (must be unique)
    pub username: Option<String>,
}

/// User response model
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
}

impl From<user::Model> for UserResponse {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
        }
    }
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = ApiResponse<UserResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserResponse>>), (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering create_user function");
    debug!("Creating user with username: {}", request.username);

    let new_user = user::ActiveModel {
        username: Set(request.username.clone()),
        ..Default::default()
    };

    trace!("Attempting to insert new user into database");
    match new_user.insert(&state.db).await {
        Ok(user_model) => {
            info!("User created successfully with ID: {}, username: {}", 
                  user_model.id, user_model.username);
            let response = ApiResponse {
                data: UserResponse::from(user_model),
                message: "User created successfully".to_string(),
                success: true,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(db_error) => {
            error!("Failed to create user '{}': {}", request.username, db_error);

            // Handle specific database errors
            let error_response = match db_error {
                DbErr::Exec(ref exec_err) => {
                    // Check for unique constraint violations
                    let error_msg = exec_err.to_string().to_lowercase();
                    if error_msg.contains("unique") || error_msg.contains("constraint") {
                        ErrorResponse {
                            error: format!("Username '{}' already exists", request.username),
                            code: "USERNAME_ALREADY_EXISTS".to_string(),
                            success: false,
                        }
                    } else {
                        ErrorResponse {
                            error: "Failed to create user due to database constraint".to_string(),
                            code: "DATABASE_CONSTRAINT_ERROR".to_string(),
                            success: false,
                        }
                    }
                }
                _ => ErrorResponse {
                    error: "Internal server error while creating user".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                    success: false,
                }
            };

            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// Get all users
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    responses(
        (status = 200, description = "Users retrieved successfully", body = ApiResponse<Vec<UserResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_users(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<UserResponse>>>, StatusCode> {
    trace!("Entering get_users function");
    debug!("Fetching all users from database");

    match user::Entity::find().all(&state.db).await {
        Ok(users) => {
            let user_count = users.len();
            debug!("Retrieved {} users from database", user_count);

            let user_responses: Vec<UserResponse> = users
                .into_iter()
                .map(UserResponse::from)
                .collect();

            info!("Successfully retrieved {} users", user_count);
            let response = ApiResponse {
                data: user_responses,
                message: "Users retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to retrieve users from database: {}", db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific user by ID
#[utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = i32, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User retrieved successfully", body = ApiResponse<UserResponse>),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn get_user(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<UserResponse>>, StatusCode> {
    trace!("Entering get_user function for user_id: {}", user_id);
    debug!("Fetching user with ID: {}", user_id);

    match user::Entity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(user_model)) => {
            info!("Successfully retrieved user with ID: {}, username: {}", 
                  user_model.id, user_model.username);
            let response = ApiResponse {
                data: UserResponse::from(user_model),
                message: "User retrieved successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Ok(None) => {
            warn!("User with ID {} not found", user_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(db_error) => {
            error!("Failed to retrieve user with ID {}: {}", user_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update a user
#[utoipa::path(
    put,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = i32, Path, description = "User ID"),
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = ApiResponse<UserResponse>),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn update_user(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, StatusCode> {
    trace!("Entering update_user function for user_id: {}", user_id);
    debug!("Updating user with ID: {}", user_id);

    // First, find the existing user
    trace!("Looking up existing user with ID: {}", user_id);
    let existing_user = match user::Entity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(user)) => {
            debug!("Found existing user: {}", user.username);
            user
        }
        Ok(None) => {
            warn!("User with ID {} not found for update", user_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(db_error) => {
            error!("Failed to lookup user with ID {} for update: {}", user_id, db_error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create active model for update
    let mut user_active: user::ActiveModel = existing_user.into();
    let mut updated_fields = Vec::new();

    // Update only provided fields
    if let Some(username) = request.username {
        debug!("Updating username to: {}", username);
        user_active.username = Set(username.clone());
        updated_fields.push(format!("username: {}", username));
    }

    if updated_fields.is_empty() {
        debug!("No fields to update for user ID: {}", user_id);
    } else {
        debug!("Updating fields: {}", updated_fields.join(", "));
    }

    trace!("Attempting to update user in database");
    match user_active.update(&state.db).await {
        Ok(updated_user) => {
            info!("User with ID {} updated successfully. Updated fields: {}", 
                  user_id, if updated_fields.is_empty() { "none".to_string() } else { updated_fields.join(", ") });
            let response = ApiResponse {
                data: UserResponse::from(updated_user),
                message: "User updated successfully".to_string(),
                success: true,
            };
            Ok(Json(response))
        }
        Err(db_error) => {
            error!("Failed to update user with ID {}: {}", user_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete a user
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = i32, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[instrument]
pub async fn delete_user(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    trace!("Entering delete_user function for user_id: {}", user_id);
    debug!("Attempting to delete user with ID: {}", user_id);

    match user::Entity::delete_by_id(user_id).exec(&state.db).await {
        Ok(delete_result) => {
            debug!("Delete operation completed. Rows affected: {}", delete_result.rows_affected);
            if delete_result.rows_affected > 0 {
                info!("User with ID {} deleted successfully", user_id);
                let response = ApiResponse {
                    data: format!("User {} deleted", user_id),
                    message: "User deleted successfully".to_string(),
                    success: true,
                };
                Ok(Json(response))
            } else {
                warn!("User with ID {} not found for deletion (no rows affected)", user_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(db_error) => {
            error!("Failed to delete user with ID {}: {}", user_id, db_error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}