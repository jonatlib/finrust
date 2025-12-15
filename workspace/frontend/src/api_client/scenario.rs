use chrono::NaiveDateTime;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioResponse {
    pub data: Scenario,
    pub message: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenariosListResponse {
    pub data: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyScenarioResponse {
    pub data: String,
    pub message: String,
    pub success: bool,
}

/// Get all scenarios
pub async fn get_scenarios() -> Result<Vec<Scenario>, String> {
    let response = Request::get("/api/v1/scenarios")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<ScenariosListResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(response.data)
}

/// Get a specific scenario by ID
pub async fn get_scenario(scenario_id: i32) -> Result<Scenario, String> {
    let url = format!("/api/v1/scenarios/{}", scenario_id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<ScenarioResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(response.data)
}

/// Create a new scenario
pub async fn create_scenario(request: CreateScenarioRequest) -> Result<Scenario, String> {
    let response = Request::post("/api/v1/scenarios")
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<ScenarioResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(response.data)
}

/// Update an existing scenario
pub async fn update_scenario(scenario_id: i32, request: UpdateScenarioRequest) -> Result<Scenario, String> {
    let url = format!("/api/v1/scenarios/{}", scenario_id);
    let response = Request::put(&url)
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<ScenarioResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(response.data)
}

/// Delete a scenario
pub async fn delete_scenario(scenario_id: i32) -> Result<(), String> {
    let url = format!("/api/v1/scenarios/{}", scenario_id);
    Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    Ok(())
}

/// Apply a scenario (convert simulated transactions to real)
pub async fn apply_scenario(scenario_id: i32) -> Result<String, String> {
    let url = format!("/api/v1/scenarios/{}/apply", scenario_id);
    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<ApplyScenarioResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(response.data)
}
