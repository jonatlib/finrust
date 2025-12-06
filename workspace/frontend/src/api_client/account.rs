use serde::{Deserialize, Serialize};
use crate::api_client;

/// Account response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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
    log::trace!("Fetching all accounts");
    let result = api_client::get("/accounts").await;
    match &result {
        Ok(accounts) => log::info!("Fetched {} accounts", accounts.len()),
        Err(e) => log::error!("Failed to fetch accounts: {}", e),
    }
    result
}

/// Get a specific account by ID
pub async fn get_account(account_id: i32) -> Result<AccountResponse, String> {
    log::trace!("Fetching account with ID: {}", account_id);
    let result = api_client::get(&format!("/accounts/{}", account_id)).await;
    match &result {
        Ok(account) => log::info!("Fetched account: {} (ID: {})", account.name, account.id),
        Err(e) => log::error!("Failed to fetch account {}: {}", account_id, e),
    }
    result
}

/// Create a new account
pub async fn create_account(request: CreateAccountRequest) -> Result<AccountResponse, String> {
    log::debug!("Creating new account: {}", request.name);
    let result = api_client::post("/accounts", &request).await;
    match &result {
        Ok(account) => log::info!("Successfully created account: {} (ID: {})", account.name, account.id),
        Err(e) => log::error!("Failed to create account '{}': {}", request.name, e),
    }
    result
}
