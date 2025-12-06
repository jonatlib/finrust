use serde::{Deserialize, Serialize};
use crate::api_client;

/// Account response model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub currency_code: String,
    pub owner_id: i32,
    pub include_in_statistics: bool,
    pub ledger_name: Option<String>,
}

/// Request body for creating a new account
#[derive(Debug, Serialize)]
pub struct CreateAccountRequest {
    pub name: String,
    pub description: Option<String>,
    pub currency_code: String,
    pub owner_id: i32,
    pub include_in_statistics: Option<bool>,
    pub ledger_name: Option<String>,
}

/// Get all accounts
pub async fn get_accounts() -> Result<Vec<AccountResponse>, String> {
    api_client::get("/accounts").await
}

/// Get a specific account by ID
pub async fn get_account(account_id: i32) -> Result<AccountResponse, String> {
    api_client::get(&format!("/accounts/{}", account_id)).await
}

/// Create a new account
pub async fn create_account(request: CreateAccountRequest) -> Result<AccountResponse, String> {
    api_client::post("/accounts", &request).await
}
