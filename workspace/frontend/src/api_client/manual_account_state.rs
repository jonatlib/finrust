use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::api_client;

/// Manual account state response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ManualAccountStateResponse {
    pub id: i32,
    pub account_id: i32,
    pub date: NaiveDate,
    pub amount: Decimal,
}

/// Request body for creating a new manual account state
#[derive(Debug, Serialize)]
pub struct CreateManualAccountStateRequest {
    pub date: NaiveDate,
    pub amount: Decimal,
}

/// Request body for updating a manual account state
#[derive(Debug, Serialize)]
pub struct UpdateManualAccountStateRequest {
    pub date: Option<NaiveDate>,
    pub amount: Option<Decimal>,
}

/// Get all manual account states for a specific account
pub async fn get_account_manual_states(account_id: i32) -> Result<Vec<ManualAccountStateResponse>, String> {
    log::trace!("Fetching manual states for account ID: {}", account_id);
    let result = api_client::get::<Vec<ManualAccountStateResponse>>(&format!("/accounts/{}/manual-states", account_id)).await;
    match &result {
        Ok(states) => log::info!("Fetched {} manual states for account ID: {}", states.len(), account_id),
        Err(e) => log::error!("Failed to fetch manual states for account {}: {}", account_id, e),
    }
    result
}

/// Get all manual account states (across all accounts)
pub async fn get_all_manual_states() -> Result<Vec<ManualAccountStateResponse>, String> {
    log::trace!("Fetching all manual account states");
    let result = api_client::get::<Vec<ManualAccountStateResponse>>("/manual-account-states").await;
    match &result {
        Ok(states) => log::info!("Fetched {} manual account states", states.len()),
        Err(e) => log::error!("Failed to fetch manual account states: {}", e),
    }
    result
}

/// Get a specific manual account state by ID
pub async fn get_manual_state(state_id: i32) -> Result<ManualAccountStateResponse, String> {
    log::trace!("Fetching manual account state with ID: {}", state_id);
    let result = api_client::get::<ManualAccountStateResponse>(&format!("/manual-account-states/{}", state_id)).await;
    match &result {
        Ok(_) => log::info!("Fetched manual account state ID: {}", state_id),
        Err(e) => log::error!("Failed to fetch manual account state {}: {}", state_id, e),
    }
    result
}

/// Create a new manual account state
pub async fn create_manual_state(account_id: i32, request: CreateManualAccountStateRequest) -> Result<ManualAccountStateResponse, String> {
    log::debug!("Creating new manual account state for account ID: {}", account_id);
    let result = api_client::post::<ManualAccountStateResponse, _>(&format!("/accounts/{}/manual-states", account_id), &request).await;
    match &result {
        Ok(state) => log::info!("Successfully created manual account state ID: {} for account ID: {}", state.id, account_id),
        Err(e) => log::error!("Failed to create manual account state for account {}: {}", account_id, e),
    }
    result
}

/// Update an existing manual account state
pub async fn update_manual_state(state_id: i32, request: UpdateManualAccountStateRequest) -> Result<ManualAccountStateResponse, String> {
    log::debug!("Updating manual account state ID: {}", state_id);
    let result = api_client::put::<ManualAccountStateResponse, _>(&format!("/manual-account-states/{}", state_id), &request).await;
    match &result {
        Ok(state) => log::info!("Successfully updated manual account state ID: {}", state.id),
        Err(e) => log::error!("Failed to update manual account state {}: {}", state_id, e),
    }
    result
}

/// Delete a manual account state
pub async fn delete_manual_state(state_id: i32) -> Result<String, String> {
    log::debug!("Deleting manual account state ID: {}", state_id);
    let result = api_client::delete::<String>(&format!("/manual-account-states/{}", state_id)).await;
    match &result {
        Ok(_) => log::info!("Successfully deleted manual account state ID: {}", state_id),
        Err(e) => log::error!("Failed to delete manual account state {}: {}", state_id, e),
    }
    result
}
