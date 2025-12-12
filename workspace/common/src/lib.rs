//! Common transport-layer types shared between backend and frontend.
//! These structs mirror the backend handlers' request/response payloads
//! so the frontend can deserialize API responses without duplicating shapes.

mod statistics;
mod timeseries;

pub use statistics::{AccountStatistics, AccountStatisticsCollection, TimePeriod};
pub use timeseries::{AccountStatePoint, AccountStateTimeseries, DateRange};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generic API response wrapper used by the backend.
/// Note: The backend has its own definition in finrust/src/schemas.rs with the
/// same field names. We mirror it here for the frontend to reuse.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Response message
    pub message: String,
    /// Success flag
    pub success: bool,
}

// ===================== Accounts =====================

/// Request body for creating a new account (mirrors backend).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CreateAccountRequest {
    pub name: String,
    pub description: Option<String>,
    pub currency_code: String,
    pub owner_id: i32,
    pub include_in_statistics: Option<bool>,
    pub ledger_name: Option<String>,
}

/// Request body for updating an account (mirrors backend).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Default)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub currency_code: Option<String>,
    pub include_in_statistics: Option<bool>,
    pub ledger_name: Option<String>,
}

/// Account response model (mirrors backend AccountResponse).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct AccountDto {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub currency_code: String,
    pub owner_id: i32,
    pub include_in_statistics: bool,
    pub ledger_name: Option<String>,
}

// ===================== Categories =====================

/// Request for creating a category.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// Request for updating a category.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Default)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// Category response model.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CategoryDto {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

// ===================== Tags =====================

/// Request for creating a tag (mirrors backend).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CreateTagRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub ledger_name: Option<String>,
}

/// Request for updating a tag (mirrors backend).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Default)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub ledger_name: Option<String>,
}

/// Tag response model (mirrors backend TagResponse).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct TagDto {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub ledger_name: Option<String>,
}
