use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{Datelike, NaiveDate};
use model::entities::{
    category, account, one_off_transaction, recurring_transaction,
    recurring_transaction_instance,
};
use compute::account::utils::generate_occurrences;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::{ToSchema, IntoParams};

/// Request structure for creating a new category
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    /// The name of the category (must be unique)
    pub name: String,
    /// Optional description of what the category is for
    pub description: Option<String>,
    /// Optional parent category ID for hierarchical categories
    pub parent_id: Option<i32>,
}

/// Request structure for updating an existing category
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

/// Yearly total for a category
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct YearlyTotal {
    pub year: i32,
    pub amount: String,
}

/// Category statistics response with tree-aggregated totals, averages, and percentages
#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryStatsResponse {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: Option<i32>,
    /// Own total (direct transactions only, excluding children)
    pub own_total: String,
    /// Total amount including children in the tree
    pub total_amount: String,
    /// Yearly totals including children
    pub yearly_totals: Vec<YearlyTotal>,
    /// Average per year including children
    pub average_per_year: String,
    /// Percentage of the absolute grand total (including children)
    pub percentage: f64,
    pub transaction_count: i64,
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
                        code: "ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
            Err(e) => {
                error!("Database error while checking parent category: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to validate parent category".to_string(),
                        code: "ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    }

    let new_category = category::ActiveModel {
        name: Set(request.name.clone()),
        description: Set(request.description),
        parent_id: Set(request.parent_id),
        ..Default::default()
    };

    match new_category.insert(&state.db).await {
        Ok(category_model) => {
            info!("Successfully created category with ID: {}", category_model.id);
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse {
                    data: CategoryResponse::from(category_model),
                    message: "Success".to_string(),
                    success: true,
                }),
            ))
        }
        Err(e) if e.to_string().contains("UNIQUE constraint failed") => {
            warn!("Category name '{}' already exists", request.name);
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!("Category with name '{}' already exists", request.name),
                    code: "CONFLICT".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to create category: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
            Ok(Json(ApiResponse {
                data: categories.into_iter().map(CategoryResponse::from).collect(),
                message: "Success".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to fetch categories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch categories".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
            Ok(Json(ApiResponse {
                data: CategoryResponse::from(category),
                message: "Success".to_string(),
                success: true,
            }))
        }
        Ok(None) => {
            warn!("Category {} not found", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Category with ID {} not found", id),
                    code: "NOT_FOUND".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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

    // Validate parent_id exists if provided and prevent circular reference
    if let Some(parent_id) = request.parent_id {
        if parent_id == id {
            warn!("Category {} cannot be its own parent", id);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Category cannot be its own parent".to_string(),
                    code: "INVALID_PARENT".to_string(),
                    success: false,
                }),
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
                        code: "ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
            Err(e) => {
                error!("Database error while checking parent category: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to validate parent category".to_string(),
                        code: "ERROR".to_string(),
                        success: false,
                    }),
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
                    code: "NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
            Ok(Json(ApiResponse {
                data: CategoryResponse::from(updated_category),
                message: "Success".to_string(),
                success: true,
            }))
        }
        Err(e) if e.to_string().contains("UNIQUE constraint failed") => {
            warn!("Category name already exists");
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Category with this name already exists".to_string(),
                    code: "CONFLICT".to_string(),
                    success: false,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to update category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
                    code: "NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Delete the category
    let category_active: category::ActiveModel = category.into();
    match category_active.delete(&state.db).await {
        Ok(_) => {
            info!("Category {} deleted successfully", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
                    code: "NOT_FOUND".to_string(),
                    success: false,
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch category {}: {}", id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch category".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Get children
    match category.get_children(&state.db).await {
        Ok(children) => {
            info!("Found {} children for category {}", children.len(), id);
            Ok(Json(ApiResponse {
                data: children.into_iter().map(CategoryResponse::from).collect(),
                message: "Success".to_string(),
                success: true,
            }))
        }
        Err(e) => {
            error!("Failed to fetch children for category {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch category children".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
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
            Json(ErrorResponse {
                error: "start_date must be before or equal to end_date".to_string(),
                code: "INVALID_DATE_RANGE".to_string(),
                success: false,
            }),
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
                        code: "ACCOUNT_NOT_FOUND".to_string(),
                        success: false,
                    }),
                ));
            }
            Err(e) => {
                error!("Failed to fetch account: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to fetch account".to_string(),
                        code: "ERROR".to_string(),
                        success: false,
                    }),
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
                    Json(ErrorResponse {
                        error: "Failed to fetch accounts".to_string(),
                        code: "ERROR".to_string(),
                        success: false,
                    }),
                ));
            }
        }
    };

    let account_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();

    // Get one-off transactions in the date range with categories
    let one_off_txns = match one_off_transaction::Entity::find()
        .filter(one_off_transaction::Column::Date.between(query.start_date, query.end_date))
        .filter(one_off_transaction::Column::CategoryId.is_not_null())
        .filter(one_off_transaction::Column::TargetAccountId.is_in(account_ids.clone()))
        .all(&state.db)
        .await
    {
        Ok(txns) => txns,
        Err(e) => {
            error!("Failed to fetch one-off transactions: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch transactions".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Get recurring transactions with categories
    let recurring_txns = match recurring_transaction::Entity::find()
        .filter(recurring_transaction::Column::CategoryId.is_not_null())
        .filter(recurring_transaction::Column::TargetAccountId.is_in(account_ids.clone()))
        .all(&state.db)
        .await
    {
        Ok(txns) => txns,
        Err(e) => {
            error!("Failed to fetch recurring transactions: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch recurring transactions".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Get recurring transaction instances in the date range for overrides
    let instances = match recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::DueDate.between(query.start_date, query.end_date))
        .all(&state.db)
        .await
    {
        Ok(insts) => insts,
        Err(e) => {
            error!("Failed to fetch recurring transaction instances: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch transaction instances".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    // Build instance lookup: (recurring_transaction_id, due_date) -> instance
    let instance_map: HashMap<(i32, NaiveDate), &recurring_transaction_instance::Model> = instances
        .iter()
        .map(|inst| ((inst.recurring_transaction_id, inst.due_date), inst))
        .collect();

    // Get all categories
    let categories = match category::Entity::find().all(&state.db).await {
        Ok(cats) => cats,
        Err(e) => {
            error!("Failed to fetch categories: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch categories".to_string(),
                    code: "ERROR".to_string(),
                    success: false,
                }),
            ));
        }
    };

    let category_map: HashMap<i32, &category::Model> = categories
        .iter()
        .map(|cat| (cat.id, cat))
        .collect();

    // Aggregate: category_id -> (yearly amounts BTreeMap<year, Decimal>, transaction_count)
    let mut stats_map: HashMap<i32, (BTreeMap<i32, Decimal>, i64)> = HashMap::new();

    // Process one-off transactions
    for txn in &one_off_txns {
        if let Some(category_id) = txn.category_id {
            let year = txn.date.year();
            let entry = stats_map.entry(category_id).or_insert_with(|| (BTreeMap::new(), 0));
            *entry.0.entry(year).or_insert(Decimal::ZERO) += txn.amount;
            entry.1 += 1;
        }
    }

    // Process recurring transactions by generating occurrences
    for rtxn in &recurring_txns {
        let occurrences = generate_occurrences(
            rtxn.start_date,
            rtxn.end_date,
            &rtxn.period,
            query.start_date,
            query.end_date,
        );

        for date in occurrences {
            // Check if there's an instance override for this occurrence
            let (amount, cat_id) = if let Some(instance) = instance_map.get(&(rtxn.id, date)) {
                // Skipped instances don't count
                if instance.status == recurring_transaction_instance::InstanceStatus::Skipped {
                    trace!("Skipping instance for recurring txn {} on {}", rtxn.id, date);
                    continue;
                }
                let amount = instance.paid_amount.unwrap_or(instance.expected_amount);
                let cat = instance.category_id.or(rtxn.category_id);
                (amount, cat)
            } else {
                (rtxn.amount, rtxn.category_id)
            };

            if let Some(category_id) = cat_id {
                let year = date.year();
                let entry = stats_map.entry(category_id).or_insert_with(|| (BTreeMap::new(), 0));
                *entry.0.entry(year).or_insert(Decimal::ZERO) += amount;
                entry.1 += 1;
            }
        }
    }

    // Build children map for tree propagation
    let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for cat in &categories {
        if let Some(parent_id) = cat.parent_id {
            children_map.entry(parent_id).or_default().push(cat.id);
        }
    }

    // Compute tree-aggregated yearly totals (own + all descendants)
    // Using post-order traversal: collect all category IDs in topological order (leaves first)
    let all_cat_ids: Vec<i32> = categories.iter().map(|c| c.id).collect();
    let topo_order = topological_sort_leaves_first(&all_cat_ids, &children_map);

    // tree_yearly: category_id -> BTreeMap<year, Decimal> (aggregated including children)
    // tree_count: category_id -> i64 (aggregated transaction count)
    let mut tree_yearly: HashMap<i32, BTreeMap<i32, Decimal>> = HashMap::new();
    let mut tree_count: HashMap<i32, i64> = HashMap::new();

    // Initialize with own stats
    for cat in &categories {
        if let Some((yearly, count)) = stats_map.get(&cat.id) {
            tree_yearly.insert(cat.id, yearly.clone());
            tree_count.insert(cat.id, *count);
        } else {
            tree_yearly.insert(cat.id, BTreeMap::new());
            tree_count.insert(cat.id, 0);
        }
    }

    // Propagate children up in leaves-first order
    for &cat_id in &topo_order {
        if let Some(parent_id) = category_map.get(&cat_id).and_then(|c| c.parent_id) {
            let child_yearly = tree_yearly.get(&cat_id).cloned().unwrap_or_default();
            let child_count = tree_count.get(&cat_id).copied().unwrap_or(0);

            let parent_yearly = tree_yearly.entry(parent_id).or_default();
            for (year, amount) in &child_yearly {
                *parent_yearly.entry(*year).or_insert(Decimal::ZERO) += amount;
            }
            *tree_count.entry(parent_id).or_insert(0) += child_count;
        }
    }

    // Calculate grand total (sum of absolute values of root categories only)
    let grand_total: Decimal = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .map(|c| {
            tree_yearly
                .get(&c.id)
                .map(|y| y.values().copied().sum::<Decimal>().abs())
                .unwrap_or(Decimal::ZERO)
        })
        .sum();

    // Determine how many distinct years appear in the date range
    let num_years = (query.start_date.year()..=query.end_date.year()).count() as i64;
    let num_years_dec = Decimal::from(num_years.max(1));

    // Build response
    let stats: Vec<CategoryStatsResponse> = categories
        .iter()
        .filter_map(|cat| {
            let own_stats = stats_map.get(&cat.id);
            let agg_yearly = tree_yearly.get(&cat.id)?;
            let agg_count = tree_count.get(&cat.id).copied().unwrap_or(0);

            // Skip categories with no transactions at all (own or aggregated)
            let agg_total: Decimal = agg_yearly.values().copied().sum();
            let own_total: Decimal = own_stats
                .map(|(y, _)| y.values().copied().sum())
                .unwrap_or(Decimal::ZERO);

            if agg_count == 0 && agg_total == Decimal::ZERO {
                return None;
            }

            let average_per_year = agg_total / num_years_dec;

            let percentage = if grand_total > Decimal::ZERO {
                use rust_decimal::prelude::ToPrimitive;
                (agg_total.abs() / grand_total * Decimal::from(100))
                    .to_f64()
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            let yearly_totals: Vec<YearlyTotal> = agg_yearly
                .iter()
                .map(|(year, amount)| YearlyTotal {
                    year: *year,
                    amount: amount.round_dp(0).to_string(),
                })
                .collect();

            Some(CategoryStatsResponse {
                category_id: cat.id,
                category_name: cat.name.clone(),
                parent_id: cat.parent_id,
                own_total: own_total.round_dp(0).to_string(),
                total_amount: agg_total.round_dp(0).to_string(),
                yearly_totals,
                average_per_year: average_per_year.round_dp(0).to_string(),
                percentage,
                transaction_count: agg_count,
            })
        })
        .collect();

    info!("Computed statistics for {} categories", stats.len());

    Ok(Json(ApiResponse {
        data: stats,
        message: "Success".to_string(),
        success: true,
    }))
}

/// Topological sort returning leaves first (post-order) for bottom-up tree propagation.
fn topological_sort_leaves_first(
    all_ids: &[i32],
    children_map: &HashMap<i32, Vec<i32>>,
) -> Vec<i32> {
    let mut result = Vec::with_capacity(all_ids.len());
    let mut visited = HashMap::new();

    fn visit(
        id: i32,
        children_map: &HashMap<i32, Vec<i32>>,
        visited: &mut HashMap<i32, bool>,
        result: &mut Vec<i32>,
    ) {
        if visited.contains_key(&id) {
            return;
        }
        visited.insert(id, true);
        if let Some(children) = children_map.get(&id) {
            for &child in children {
                visit(child, children_map, visited, result);
            }
        }
        result.push(id);
    }

    for &id in all_ids {
        visit(id, children_map, &mut visited, &mut result);
    }

    result
}
