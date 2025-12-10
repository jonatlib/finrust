use serde::{Deserialize, Serialize};
use crate::api_client;
use rust_decimal::Decimal;

/// Recurrence period enum (matching backend)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RecurrencePeriod {
    Daily,
    Weekly,
    WorkDay,
    Monthly,
    Quarterly,
    HalfYearly,
    Yearly,
}

impl RecurrencePeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecurrencePeriod::Daily => "Daily",
            RecurrencePeriod::Weekly => "Weekly",
            RecurrencePeriod::WorkDay => "WorkDay",
            RecurrencePeriod::Monthly => "Monthly",
            RecurrencePeriod::Quarterly => "Quarterly",
            RecurrencePeriod::HalfYearly => "HalfYearly",
            RecurrencePeriod::Yearly => "Yearly",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RecurrencePeriod::Daily => "Daily",
            RecurrencePeriod::Weekly => "Weekly",
            RecurrencePeriod::WorkDay => "Work Days (Mon-Fri)",
            RecurrencePeriod::Monthly => "Monthly",
            RecurrencePeriod::Quarterly => "Quarterly",
            RecurrencePeriod::HalfYearly => "Half-Yearly",
            RecurrencePeriod::Yearly => "Yearly",
        }
    }
}

/// Instance status enum (matching backend)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum InstanceStatus {
    Pending,
    Paid,
    Skipped,
}

impl InstanceStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            InstanceStatus::Pending => "Pending",
            InstanceStatus::Paid => "Paid",
            InstanceStatus::Skipped => "Skipped",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            InstanceStatus::Pending => "badge-warning",
            InstanceStatus::Paid => "badge-success",
            InstanceStatus::Skipped => "badge-ghost",
        }
    }
}

/// Tag information
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TagInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Recurring transaction response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RecurringTransactionResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub amount: String, // Decimal as string from API
    pub start_date: String, // NaiveDate as string
    pub end_date: Option<String>,
    pub period: String,
    pub include_in_statistics: bool,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
    pub tags: Vec<TagInfo>,
}

/// Recurring transaction instance response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RecurringInstanceResponse {
    pub id: i32,
    pub recurring_transaction_id: i32,
    pub recurring_transaction_name: Option<String>,
    pub target_account_id: Option<i32>,
    pub target_account_name: Option<String>,
    pub source_account_id: Option<i32>,
    pub source_account_name: Option<String>,
    pub status: String,
    pub due_date: String,
    pub expected_amount: String,
    pub paid_date: Option<String>,
    pub paid_amount: Option<String>,
    pub reconciled_imported_transaction_id: Option<i32>,
    pub tags: Vec<TagInfo>,
}

/// Request body for creating a recurring transaction
#[derive(Debug, Serialize)]
pub struct CreateRecurringTransactionRequest {
    pub name: String,
    pub description: Option<String>,
    pub amount: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub period: String,
    pub include_in_statistics: Option<bool>,
    pub target_account_id: i32,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
}

/// Request body for updating a recurring transaction
#[derive(Debug, Serialize)]
pub struct UpdateRecurringTransactionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub amount: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period: Option<String>,
    pub include_in_statistics: Option<bool>,
    pub target_account_id: Option<i32>,
    pub source_account_id: Option<i32>,
    pub ledger_name: Option<String>,
}

/// Request body for creating a recurring transaction instance
#[derive(Debug, Serialize)]
pub struct CreateRecurringInstanceRequest {
    pub date: String,
    pub amount: Option<String>,
}

/// Get all recurring transactions
pub async fn get_recurring_transactions(
    page: Option<u64>,
    limit: Option<u64>,
    target_account_id: Option<i32>,
    source_account_id: Option<i32>,
) -> Result<Vec<RecurringTransactionResponse>, String> {
    log::trace!("Fetching recurring transactions");

    let mut query_params = Vec::new();
    if let Some(p) = page {
        query_params.push(format!("page={}", p));
    }
    if let Some(l) = limit {
        query_params.push(format!("limit={}", l));
    }
    if let Some(target_id) = target_account_id {
        query_params.push(format!("target_account_id={}", target_id));
    }
    if let Some(source_id) = source_account_id {
        query_params.push(format!("source_account_id={}", source_id));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    let result = api_client::get::<Vec<RecurringTransactionResponse>>(
        &format!("/recurring-transactions{}", query_string)
    ).await;

    match &result {
        Ok(transactions) => log::info!("Fetched {} recurring transactions", transactions.len()),
        Err(e) => log::error!("Failed to fetch recurring transactions: {}", e),
    }
    result
}

/// Get a specific recurring transaction by ID
pub async fn get_recurring_transaction(id: i32) -> Result<RecurringTransactionResponse, String> {
    log::trace!("Fetching recurring transaction with ID: {}", id);
    let result = api_client::get::<RecurringTransactionResponse>(
        &format!("/recurring-transactions/{}", id)
    ).await;

    match &result {
        Ok(transaction) => log::info!("Fetched recurring transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to fetch recurring transaction {}: {}", id, e),
    }
    result
}

/// Create a new recurring transaction
pub async fn create_recurring_transaction(
    request: CreateRecurringTransactionRequest
) -> Result<RecurringTransactionResponse, String> {
    log::debug!("Creating new recurring transaction: {}", request.name);
    let result = api_client::post::<RecurringTransactionResponse, _>(
        "/recurring-transactions",
        &request
    ).await;

    match &result {
        Ok(transaction) => log::info!("Successfully created recurring transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to create recurring transaction '{}': {}", request.name, e),
    }
    result
}

/// Update an existing recurring transaction
pub async fn update_recurring_transaction(
    id: i32,
    request: UpdateRecurringTransactionRequest
) -> Result<RecurringTransactionResponse, String> {
    log::debug!("Updating recurring transaction ID: {}", id);
    let result = api_client::put::<RecurringTransactionResponse, _>(
        &format!("/recurring-transactions/{}", id),
        &request
    ).await;

    match &result {
        Ok(transaction) => log::info!("Successfully updated recurring transaction: {} (ID: {})", transaction.name, transaction.id),
        Err(e) => log::error!("Failed to update recurring transaction {}: {}", id, e),
    }
    result
}

/// Delete a recurring transaction
pub async fn delete_recurring_transaction(id: i32) -> Result<String, String> {
    log::debug!("Deleting recurring transaction ID: {}", id);
    let result = api_client::delete::<String>(
        &format!("/recurring-transactions/{}", id)
    ).await;

    match &result {
        Ok(_) => log::info!("Successfully deleted recurring transaction ID: {}", id),
        Err(e) => log::error!("Failed to delete recurring transaction {}: {}", id, e),
    }
    result
}

/// Create a new recurring transaction instance
pub async fn create_recurring_instance(
    recurring_transaction_id: i32,
    request: CreateRecurringInstanceRequest
) -> Result<RecurringInstanceResponse, String> {
    log::debug!("Creating instance for recurring transaction ID: {}", recurring_transaction_id);
    let result = api_client::post::<RecurringInstanceResponse, _>(
        &format!("/recurring-transactions/{}/instances", recurring_transaction_id),
        &request
    ).await;

    match &result {
        Ok(instance) => log::info!("Successfully created instance with ID: {}", instance.id),
        Err(e) => log::error!("Failed to create instance for recurring transaction {}: {}", recurring_transaction_id, e),
    }
    result
}
/// Get all recurring transaction instances
pub async fn get_recurring_instances(
    page: Option<u64>,
    limit: Option<u64>,
    recurring_transaction_id: Option<i32>,
    status: Option<String>,
) -> Result<Vec<RecurringInstanceResponse>, String> {
    log::trace!("Fetching recurring instances");

    let mut query_params = Vec::new();
    if let Some(p) = page {
        query_params.push(format!("page={}", p));
    }
    if let Some(l) = limit {
        query_params.push(format!("limit={}", l));
    }
    if let Some(rt_id) = recurring_transaction_id {
        query_params.push(format!("recurring_transaction_id={}", rt_id));
    }
    if let Some(s) = status {
        query_params.push(format!("status={}", s));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    let result = api_client::get::<Vec<RecurringInstanceResponse>>(
        &format!("/recurring-instances{}", query_string)
    ).await;

    match &result {
        Ok(instances) => log::info!("Fetched {} recurring instances", instances.len()),
        Err(e) => log::error!("Failed to fetch recurring instances: {}", e),
    }
    result
}

/// Get a specific recurring transaction instance by ID
pub async fn get_recurring_instance(id: i32) -> Result<RecurringInstanceResponse, String> {
    log::trace!("Fetching recurring instance with ID: {}", id);
    let result = api_client::get::<RecurringInstanceResponse>(
        &format!("/recurring-instances/{}", id)
    ).await;

    match &result {
        Ok(instance) => log::info!("Fetched recurring instance with ID: {}", instance.id),
        Err(e) => log::error!("Failed to fetch recurring instance {}: {}", id, e),
    }
    result
}

/// Request body for updating a recurring transaction instance
#[derive(Debug, Serialize)]
pub struct UpdateRecurringInstanceRequest {
    pub status: Option<String>,
    pub due_date: Option<String>,
    pub expected_amount: Option<String>,
    pub paid_date: Option<String>,
    pub paid_amount: Option<String>,
}

/// Update an existing recurring transaction instance
pub async fn update_recurring_instance(
    id: i32,
    request: UpdateRecurringInstanceRequest
) -> Result<RecurringInstanceResponse, String> {
    log::debug!("Updating recurring instance ID: {}", id);
    let result = api_client::put::<RecurringInstanceResponse, _>(
        &format!("/recurring-instances/{}", id),
        &request
    ).await;

    match &result {
        Ok(instance) => log::info!("Successfully updated recurring instance with ID: {}", instance.id),
        Err(e) => log::error!("Failed to update recurring instance {}: {}", id, e),
    }
    result
}

/// Delete a recurring transaction instance
pub async fn delete_recurring_instance(id: i32) -> Result<String, String> {
    log::debug!("Deleting recurring instance ID: {}", id);
    let result = api_client::delete::<String>(
        &format!("/recurring-instances/{}", id)
    ).await;

    match &result {
        Ok(_) => log::info!("Successfully deleted recurring instance ID: {}", id),
        Err(e) => log::error!("Failed to delete recurring instance {}: {}", id, e),
    }
    result
}
