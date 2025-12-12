use serde::{Deserialize, Serialize};
use crate::api_client;

/// Category response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// Request body for creating a new category
#[derive(Debug, Serialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// Request body for updating a category
#[derive(Debug, Serialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// Category statistics response
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryStatistics {
    pub category_id: i32,
    pub category_name: String,
    pub total_amount: String,
    pub transaction_count: i64,
}

/// Get all categories
pub async fn get_categories() -> Result<Vec<CategoryResponse>, String> {
    log::trace!("Fetching all categories");
    let result = api_client::get::<Vec<CategoryResponse>>("/categories").await;
    match &result {
        Ok(categories) => log::info!("Fetched {} categories", categories.len()),
        Err(e) => log::error!("Failed to fetch categories: {}", e),
    }
    result
}

/// Get a specific category by ID
pub async fn get_category(category_id: i32) -> Result<CategoryResponse, String> {
    log::trace!("Fetching category with ID: {}", category_id);
    let result = api_client::get::<CategoryResponse>(&format!("/categories/{}", category_id)).await;
    match &result {
        Ok(category) => log::info!("Fetched category: {} (ID: {})", category.name, category.id),
        Err(e) => log::error!("Failed to fetch category {}: {}", category_id, e),
    }
    result
}

/// Create a new category
pub async fn create_category(request: CreateCategoryRequest) -> Result<CategoryResponse, String> {
    log::debug!("Creating new category: {}", request.name);
    let result = api_client::post::<CategoryResponse, _>("/categories", &request).await;
    match &result {
        Ok(category) => log::info!("Successfully created category: {} (ID: {})", category.name, category.id),
        Err(e) => log::error!("Failed to create category '{}': {}", request.name, e),
    }
    result
}

/// Update an existing category
pub async fn update_category(category_id: i32, request: UpdateCategoryRequest) -> Result<CategoryResponse, String> {
    log::debug!("Updating category ID: {}", category_id);
    let result = api_client::put::<CategoryResponse, _>(&format!("/categories/{}", category_id), &request).await;
    match &result {
        Ok(category) => log::info!("Successfully updated category: {} (ID: {})", category.name, category.id),
        Err(e) => log::error!("Failed to update category {}: {}", category_id, e),
    }
    result
}

/// Delete a category
pub async fn delete_category(category_id: i32) -> Result<String, String> {
    log::debug!("Deleting category ID: {}", category_id);
    let result = api_client::delete::<String>(&format!("/categories/{}", category_id)).await;
    match &result {
        Ok(_) => log::info!("Successfully deleted category ID: {}", category_id),
        Err(e) => log::error!("Failed to delete category {}: {}", category_id, e),
    }
    result
}

/// Get children of a category
pub async fn get_category_children(category_id: i32) -> Result<Vec<CategoryResponse>, String> {
    log::trace!("Fetching children for category ID: {}", category_id);
    let result = api_client::get::<Vec<CategoryResponse>>(&format!("/categories/{}/children", category_id)).await;
    match &result {
        Ok(children) => log::info!("Fetched {} children for category ID: {}", children.len(), category_id),
        Err(e) => log::error!("Failed to fetch children for category {}: {}", category_id, e),
    }
    result
}

/// Get category statistics
pub async fn get_category_stats(start_date: &str, end_date: &str) -> Result<Vec<CategoryStatistics>, String> {
    log::trace!("Fetching category statistics from {} to {}", start_date, end_date);
    let endpoint = format!("/categories/stats?start_date={}&end_date={}", start_date, end_date);
    let result = api_client::get::<Vec<CategoryStatistics>>(&endpoint).await;
    match &result {
        Ok(stats) => log::info!("Fetched statistics for {} categories", stats.len()),
        Err(e) => log::error!("Failed to fetch category statistics: {}", e),
    }
    result
}
