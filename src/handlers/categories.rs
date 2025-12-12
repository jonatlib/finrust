use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDate;
use compute::categories::CategoriesComputer;
use model::entities::{category, account};
use model::transaction::TransactionGenerator;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, DbErr, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug};
use utoipa::{ToSchema, IntoParams};

/// Request structure for creating a new category
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    /// The name of the category (must be unique)
    pub name: String,
    /// Optional description of what the category is for
    pub description: Option<String>,
    /// Optional parent category ID for hierarchical categories
    pub parent_id: Option<i32>,
}

/// Request structure for updating an existing category
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCategoryRequest {
    /// The name of the category (must be unique)
    pub name: Option<String>,
    /// Optional description of what the category is for
    pub description: Option<String>,
    /// Optional parent category ID for hierarchical categories
    pub parent_id: Option<i32>,
}

/// Response structure for category operations
#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

impl From<category::Model> for CategoryResponse {
    fn from(model: category::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            parent_id: model.parent_id,
        }
    }
}

/// Query parameters for category statistics
#[derive(Debug, Deserialize, IntoParams)]
pub struct CategoryStatsQuery {
    /// Start date for statistics (inclusive)
    pub start_date: NaiveDate,
    /// End date for statistics (inclusive)
    pub end_date: NaiveDate,
    /// Optional account ID to filter by specific account
    pub account_id: Option<i32>,
}

/// Category statistics response
#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryStatsResponse {
    pub date: String,
    pub account: i32,
    pub category_id: i32,
    pub category_name: String,
    pub amount: f64,
}

/// Create a new category
#[utoipa::path(
    post,
    path = "/api/v1/categories",
    request_body = CreateCategoryRequest,
    responses(
        (status = 201, description = "Category created successfully", body = ApiResponse<CategoryResponse>),
        (status = 400, description = "Invalid request data", body = ErrorResponse),
        (status = 409, description = "Category name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn create_category(
    State(state): State<AppState>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CategoryResponse>>), (StatusCode, Json<ErrorResponse>)> {
    debug!("Creating category with name: {}", request.name);

    // Validate parent_id exists if provided
    if let Some(parent_id) = request.parent_id {
        match category::Entity::find_by_id(parent_id).one(&state.db).await {
            Ok(Some(_)) => {
                debug!("Parent category {} exists", parent_id);
            }
            Ok(None) => {
                warn!("Parent category {} not found", parent_id);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Parent category with ID {} not found", parent_id),
                        code: "ERROR".to_string(), success: false },
                    }),
                ));
            }
            Err(e) => {
                error!("Database error checking parent category: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: "Failed to validate parent category".to_string(), code: "ERROR".to_string()success: false }),
                ));
            }
        }
    }

    // Create the category
    let new_category = category::ActiveModel {
        name: Set(request.name.clone()),
        description: Set(request.description),
        parent_id: Set(request.parent_id),
        ..Default::default()
    };

    match new_category.insert(&state.db).await {
        Ok(category) => {
            info!("Category created successfully with ID: {}", category.id);
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse { data: CategoryResponse::from(category),
                , message: String::new(), success: true }),
            ))
        }
        Err(DbErr::Exec(_)) => {
            warn!("Category name '{}' already exists", request.name);
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!("Category with name '{}' already exists", request.name),
                    code: "DUPLICATE_CATEGORY".to_string(), success: false },
                }),
            ))
        }
        Err(e) => {
            error!("Failed to create category: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to create category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Get all categories
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    responses(
        (status = 200, description = "List of all categories", body = ApiResponse<Vec<CategoryResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn get_categories(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CategoryResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Fetching all categories");

    match category::Entity::find().all(&state.db).await {
        Ok(categories) => {
            info!("Retrieved {} categories", categories.len());
            Ok(Json(ApiResponse { data: categories.into_iter().map(CategoryResponse::from).collect(), message: String::new(), success: true }))
        }
        Err(e) => {
            error!("Failed to fetch categories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch categories".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Get a single category by ID
#[utoipa::path(
    get,
    path = "/api/v1/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Category found", body = ApiResponse<CategoryResponse>),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn get_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CategoryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Fetching category with ID: {}", id);

    match category::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(category)) => {
            info!("Category {} found", id);
            Ok(Json(ApiResponse { data: CategoryResponse::from(category), message: String::new(), success: true }))
        }
        Ok(None) => {
            warn!("Category {} not found", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Category with ID {} not found", id),
                    code: "NOT_FOUND".to_string(), success: false },
                }),
            ))
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Update a category
#[utoipa::path(
    put,
    path = "/api/v1/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    request_body = UpdateCategoryRequest,
    responses(
        (status = 200, description = "Category updated successfully", body = ApiResponse<CategoryResponse>),
        (status = 400, description = "Invalid request data", body = ErrorResponse),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 409, description = "Category name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<Json<ApiResponse<CategoryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Updating category with ID: {}", id);

    // Validate parent_id exists if provided
    if let Some(parent_id) = request.parent_id {
        // Prevent circular reference
        if parent_id == id {
            warn!("Category {} cannot be its own parent", id);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: "Category cannot be its own parent".to_string(), code: "INVALID_PARENT".to_string()success: false }),
            ));
        }

        match category::Entity::find_by_id(parent_id).one(&state.db).await {
            Ok(Some(_)) => {
                debug!("Parent category {} exists", parent_id);
            }
            Ok(None) => {
                warn!("Parent category {} not found", parent_id);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Parent category with ID {} not found", parent_id),
                        code: "ERROR".to_string(), success: false },
                    }),
                ));
            }
            Err(e) => {
                error!("Database error checking parent category: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: "Failed to validate parent category".to_string(), code: "ERROR".to_string()success: false }),
                ));
            }
        }
    }

    // Find the existing category
    let existing_category = match category::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(cat)) => cat,
        Ok(None) => {
            warn!("Category {} not found", id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Category with ID {} not found", id),
                    code: "NOT_FOUND".to_string(), success: false },
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ));
        }
    };

    // Update the category
    let mut category: category::ActiveModel = existing_category.into();

    if let Some(name) = request.name {
        category.name = Set(name);
    }
    if request.description.is_some() {
        category.description = Set(request.description);
    }
    if request.parent_id.is_some() {
        category.parent_id = Set(request.parent_id);
    }

    match category.update(&state.db).await {
        Ok(updated_category) => {
            info!("Category {} updated successfully", id);
            Ok(Json(ApiResponse { data: CategoryResponse::from(updated_category), message: String::new(), success: true }))
        }
        Err(DbErr::Exec(_)) => {
            warn!("Category name already exists");
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse { error: "Category with this name already exists".to_string(), code: "DUPLICATE_CATEGORY".to_string()success: false }),
            ))
        }
        Err(e) => {
            error!("Failed to update category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to update category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Delete a category
#[utoipa::path(
    delete,
    path = "/api/v1/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 204, description = "Category deleted successfully"),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("Deleting category with ID: {}", id);

    // Find the category
    let category = match category::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(cat)) => cat,
        Ok(None) => {
            warn!("Category {} not found", id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Category with ID {} not found", id),
                    code: "NOT_FOUND".to_string(), success: false },
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ));
        }
    };

    // Delete the category
    match category.delete(&state.db).await {
        Ok(_) => {
            info!("Category {} deleted successfully", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to delete category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Get children of a category
#[utoipa::path(
    get,
    path = "/api/v1/categories/{id}/children",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "List of child categories", body = ApiResponse<Vec<CategoryResponse>>),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn get_category_children(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<CategoryResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Fetching children for category {}", id);

    // Verify the category exists
    let category = match category::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(cat)) => cat,
        Ok(None) => {
            warn!("Category {} not found", id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Category with ID {} not found", id),
                    code: "NOT_FOUND".to_string(), success: false },
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch category".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ));
        }
    };

    // Get children
    match category.get_children(&state.db).await {
        Ok(children) => {
            info!("Found {} children for category {}", children.len(), id);
            Ok(Json(ApiResponse { data: children.into_iter().map(CategoryResponse::from).collect(), message: String::new(), success: true }))
        }
        Err(e) => {
            error!("Failed to fetch children for category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to fetch category children".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}

/// Get category statistics
#[utoipa::path(
    get,
    path = "/api/v1/categories/stats",
    params(CategoryStatsQuery),
    responses(
        (status = 200, description = "Category statistics", body = ApiResponse<Vec<CategoryStatsResponse>>),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "categories"
)]
#[instrument(skip(state))]
pub async fn get_category_stats(
    State(state): State<AppState>,
    Query(query): Query<CategoryStatsQuery>,
) -> Result<Json<ApiResponse<Vec<CategoryStatsResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    debug!(
        "Fetching category stats from {} to {}",
        query.start_date, query.end_date
    );

    // Validate date range
    if query.start_date > query.end_date {
        warn!("Invalid date range: start_date > end_date");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "start_date must be before or equal to end_date".to_string(), code: "INVALID_DATE_RANGE".to_string()success: false }),
        ));
    }

    // Get all accounts or filter by account_id
    let accounts = if let Some(account_id) = query.account_id {
        match account::Entity::find_by_id(account_id).one(&state.db).await {
            Ok(Some(acc)) => vec![acc],
            Ok(None) => {
                warn!("Account {} not found", account_id);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Account with ID {} not found", account_id),
                        code: "ACCOUNT_NOT_FOUND".to_string(), success: false },
                    }),
                ));
            }
            Err(e) => {
                error!("Failed to fetch account: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: "Failed to fetch account".to_string(), code: "ERROR".to_string()success: false }),
                ));
            }
        }
    } else {
        match account::Entity::find().all(&state.db).await {
            Ok(accounts) => accounts,
            Err(e) => {
                error!("Failed to fetch accounts: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: "Failed to fetch accounts".to_string(), code: "ERROR".to_string()success: false }),
                ));
            }
        }
    };

    // Generate transactions for all accounts
    let mut all_transactions = Vec::new();
    for account in accounts {
        match account
            .generate_transactions(&state.db, query.start_date, query.end_date)
            .await
        {
            Ok(transactions) => {
                debug!(
                    "Generated {} transactions for account {}",
                    transactions.len(),
                    account.id
                );
                all_transactions.extend(transactions);
            }
            Err(e) => {
                error!("Failed to generate transactions for account {}: {}", account.id, e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to generate transactions for account {}", account.id),
                        code: "ERROR".to_string(), success: false },
                    }),
                ));
            }
        }
    }

    // Compute category statistics
    let computer = CategoriesComputer::new();
    match computer
        .compute_categories_summary(&state.db, all_transactions, query.start_date, query.end_date)
        .await
    {
        Ok(df) => {
            info!("Computed category statistics with {} rows", df.height());

            // Convert DataFrame to response format
            let mut stats = Vec::new();
            for i in 0..df.height() {
                let date_ms = df.column("date").unwrap().i64().unwrap().get(i).unwrap();
                let date = chrono::DateTime::from_timestamp_millis(date_ms)
                    .unwrap()
                    .naive_utc()
                    .date()
                    .to_string();
                let account = df.column("account").unwrap().i32().unwrap().get(i).unwrap();
                let category_id = df.column("category_id").unwrap().i32().unwrap().get(i).unwrap();
                let category_name = df
                    .column("category_name")
                    .unwrap()
                    .str()
                    .unwrap()
                    .get(i)
                    .unwrap()
                    .to_string();
                let amount = df.column("amount").unwrap().f64().unwrap().get(i).unwrap();

                stats.push(CategoryStatsResponse {
                    date,
                    account,
                    category_id,
                    category_name,
                    amount,
                });
            }

            Ok(Json(ApiResponse { data: stats , message: String::new(), success: true }))
        }
        Err(e) => {
            error!("Failed to compute category statistics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Failed to compute category statistics".to_string(), code: "ERROR".to_string(), success: false },
                success: false }),
            ))
        }
    }
}
