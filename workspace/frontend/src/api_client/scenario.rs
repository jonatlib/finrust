use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use crate::api_client;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScenarioRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScenarioRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// Get all scenarios
pub async fn get_scenarios() -> Result<Vec<Scenario>, String> {
    log::trace!("Fetching all scenarios");
    let result = api_client::get::<Vec<Scenario>>("/scenarios").await;
    match &result {
        Ok(scenarios) => log::info!("Fetched {} scenarios", scenarios.len()),
        Err(e) => log::error!("Failed to fetch scenarios: {}", e),
    }
    result
}

/// Get a specific scenario by ID
pub async fn get_scenario(scenario_id: i32) -> Result<Scenario, String> {
    log::trace!("Fetching scenario with ID: {}", scenario_id);
    let url = format!("/scenarios/{}", scenario_id);
    let result = api_client::get::<Scenario>(&url).await;
    match &result {
        Ok(scenario) => log::info!("Fetched scenario: {} (ID: {})", scenario.name, scenario.id),
        Err(e) => log::error!("Failed to fetch scenario {}: {}", scenario_id, e),
    }
    result
}

/// Create a new scenario
pub async fn create_scenario(request: CreateScenarioRequest) -> Result<Scenario, String> {
    log::debug!("Creating new scenario: {}", request.name);
    let result = api_client::post::<Scenario, _>("/scenarios", &request).await;
    match &result {
        Ok(scenario) => log::info!("Created scenario: {} (ID: {})", scenario.name, scenario.id),
        Err(e) => log::error!("Failed to create scenario: {}", e),
    }
    result
}

/// Update an existing scenario
pub async fn update_scenario(scenario_id: i32, request: UpdateScenarioRequest) -> Result<Scenario, String> {
    log::debug!("Updating scenario {}", scenario_id);
    let url = format!("/scenarios/{}", scenario_id);
    let result = api_client::put::<Scenario, _>(&url, &request).await;
    match &result {
        Ok(scenario) => log::info!("Updated scenario: {} (ID: {})", scenario.name, scenario.id),
        Err(e) => log::error!("Failed to update scenario {}: {}", scenario_id, e),
    }
    result
}

/// Delete a scenario
pub async fn delete_scenario(scenario_id: i32) -> Result<(), String> {
    log::debug!("Deleting scenario {}", scenario_id);
    let url = format!("/scenarios/{}", scenario_id);

    // For DELETE with no response body (204 No Content), we need to handle it differently
    use gloo_net::http::Request;
    use crate::settings;

    let api_base = settings::get_settings().api_base_url();
    let full_url = format!("{}{}", api_base, url);

    let response = Request::delete(&full_url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Request failed: {}", e);
            log::error!("DELETE {} - {}", url, error_msg);
            error_msg
        })?;

    if !response.ok() {
        let error_msg = format!("HTTP error: {}", response.status());
        log::error!("DELETE {} - {}", url, error_msg);
        return Err(error_msg);
    }

    log::info!("Deleted scenario ID: {}", scenario_id);
    Ok(())
}

/// Apply a scenario (convert simulated transactions to real)
pub async fn apply_scenario(scenario_id: i32) -> Result<String, String> {
    log::debug!("Applying scenario {}", scenario_id);
    let url = format!("/scenarios/{}/apply", scenario_id);

    // POST with empty body
    #[derive(Serialize)]
    struct EmptyBody {}

    let result = api_client::post::<String, _>(&url, &EmptyBody {}).await;
    match &result {
        Ok(message) => log::info!("Applied scenario {}: {}", scenario_id, message),
        Err(e) => log::error!("Failed to apply scenario {}: {}", scenario_id, e),
    }
    result
}
