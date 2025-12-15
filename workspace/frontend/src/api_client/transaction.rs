use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::api_client;

/// Tag information for API responses
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TagInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Transaction response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TransactionResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub include_in_statistics: bool,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
    pub linked_import_id: Option<String>,
    pub category_id: Option<i32>,
    pub tags: Vec<TagInfo>,
    pub scenario_id: Option<i32>,
    pub is_simulated: bool,
}

/// Request body for creating a new transaction
#[derive(Debug, Serialize)]
pub struct CreateTransactionRequest {
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub include_in_statistics: Option<bool>,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
    pub linked_import_id: Option<String>,
    pub category_id: Option<i32>,
    pub scenario_id: Option<i32>,
    pub is_simulated: Option<bool>,
}

/// Request body for updating a transaction
#[derive(Debug, Serialize)]
pub struct UpdateTransactionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub amount: Option<Decimal>,
    pub date: Option<NaiveDate>,
    pub include_in_statistics: Option<bool>,
    pub target_account_id: Option<i32>,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
    pub linked_import_id: Option<String>,
    pub category_id: Option<i32>,
    pub scenario_id: Option<i32>,
    pub is_simulated: Option<bool>,
}

/// Get all transactions
pub async fn get_transactions() -> Result<Vec<TransactionResponse>, String> {
    log::trace!("Fetching all transactions");
    let result = api_client::get::<Vec<TransactionResponse>>("/transactions").await;
    match &result {
        Ok(transactions) => log::info!("Fetched {} transactions", transactions.len()),
        Err(e) => log::error!("Failed to fetch transactions: {}", e),
    }
    result
}

/// Get a specific transaction by ID
pub async fn get_transaction(transaction_id: i32) -> Result<TransactionResponse, String> {
    log::trace!("Fetching transaction with ID: {}", transaction_id);
    let result = api_client::get::<TransactionResponse>(&format!("/transactions/{}", transaction_id)).await;
    match &result {
        Ok(transaction) => log::info!("Fetched transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to fetch transaction {}: {}", transaction_id, e),
    }
    result
}

/// Get transactions for a specific account
pub async fn get_account_transactions(account_id: i32) -> Result<Vec<TransactionResponse>, String> {
    log::trace!("Fetching transactions for account ID: {}", account_id);
    let result = api_client::get::<Vec<TransactionResponse>>(&format!("/accounts/{}/transactions", account_id)).await;
    match &result {
        Ok(transactions) => log::info!("Fetched {} transactions for account ID: {}", transactions.len(), account_id),
        Err(e) => log::error!("Failed to fetch transactions for account {}: {}", account_id, e),
    }
    result
}

/// Create a new transaction
pub async fn create_transaction(request: CreateTransactionRequest) -> Result<TransactionResponse, String> {
    log::debug!("Creating new transaction: {}", request.name);
    let result = api_client::post::<TransactionResponse, _>("/transactions", &request).await;
    match &result {
        Ok(transaction) => log::info!("Successfully created transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to create transaction '{}': {}", request.name, e),
    }
    result
}

/// Update an existing transaction
pub async fn update_transaction(transaction_id: i32, request: UpdateTransactionRequest) -> Result<TransactionResponse, String> {
    log::debug!("Updating transaction ID: {}", transaction_id);
    let result = api_client::put::<TransactionResponse, _>(&format!("/transactions/{}", transaction_id), &request).await;
    match &result {
        Ok(transaction) => log::info!("Successfully updated transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to update transaction {}: {}", transaction_id, e),
    }
    result
}

/// Delete a transaction
pub async fn delete_transaction(transaction_id: i32) -> Result<String, String> {
    log::debug!("Deleting transaction ID: {}", transaction_id);
    let result = api_client::delete::<String>(&format!("/transactions/{}", transaction_id)).await;
    match &result {
        Ok(_) => log::info!("Successfully deleted transaction ID: {}", transaction_id),
        Err(e) => log::error!("Failed to delete transaction {}: {}", transaction_id, e),
    }
    result
}
