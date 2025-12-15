use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveDateTime;
use model::entities::{one_off_transaction, recurring_income, recurring_transaction, scenario};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, trace, warn};
use utoipa::{IntoParams, ToSchema};

/// Request body for creating a scenario
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateScenarioRequest {
    /// Name of the scenario (e.g., "Buy Tesla", "Buy Toyota")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
}

/// Request body for updating a scenario
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateScenarioRequest {
    /// Name of the scenario
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Whether this scenario is active
    pub is_active: Option<bool>,
}

/// Scenario response model
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScenarioResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub is_active: bool,
}

impl From<scenario::Model> for ScenarioResponse {
    fn from(model: scenario::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            created_at: model.created_at,
            is_active: model.is_active,
        }
    }
}

/// Query parameters for listing scenarios
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListScenariosQuery {
    /// Page number (starting from 0)
    pub page: Option<u64>,
    /// Page size (default: 50)
    pub page_size: Option<u64>,
}

/// Create a new scenario
///
/// Creates a new what-if scenario that can contain simulated transactions.
#[utoipa::path(
    post,
    path = "/api/v1/scenarios",
    request_body = CreateScenarioRequest,
    responses(
        (status = 201, description = "Scenario created successfully", body = ScenarioResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn create_scenario(
    State(state): State<AppState>,
    Json(request): Json<CreateScenarioRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ScenarioResponse>>), (StatusCode, Json<ErrorResponse>)>
{
    trace!("Entering create_scenario function");
    debug!("Creating scenario: {:?}", request);

    let db = &state.db;

    // Create the scenario
    let scenario = scenario::ActiveModel {
        name: Set(request.name),
        description: Set(request.description),
        created_at: Set(chrono::Local::now().naive_local()),
        is_active: Set(false),
        ..Default::default()
    };

    let result = scenario.insert(db).await.map_err(|e| {
        error!("Failed to create scenario: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                    error: format!("Failed to create scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
        )
    })?;

    info!("Scenario created successfully: id={}", result.id);
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: result.into(),
            message: "Scenario created successfully".to_string(),
            success: true,
        }),
    ))
}

/// Get all scenarios
///
/// Returns a paginated list of all what-if scenarios.
#[utoipa::path(
    get,
    path = "/api/v1/scenarios",
    params(ListScenariosQuery),
    responses(
        (status = 200, description = "List of scenarios", body = Vec<ScenarioResponse>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn get_scenarios(
    State(state): State<AppState>,
    Query(query): Query<ListScenariosQuery>,
) -> Result<Json<ApiResponse<Vec<ScenarioResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_scenarios function");
    debug!("Query parameters: {:?}", query);

    let db = &state.db;
    let page = query.page.unwrap_or(0);
    let page_size = query.page_size.unwrap_or(50);

    let scenarios = scenario::Entity::find()
        .order_by_desc(scenario::Column::CreatedAt)
        .paginate(db, page_size)
        .fetch_page(page)
        .await
        .map_err(|e| {
            error!("Failed to fetch scenarios: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch scenarios: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    let responses: Vec<ScenarioResponse> = scenarios.into_iter().map(|s| s.into()).collect();

    info!("Fetched {} scenarios", responses.len());
    Ok(Json(ApiResponse {
        data: responses,
        message: "Scenarios retrieved successfully".to_string(),
        success: true,
    }))
}

/// Get a specific scenario by ID
///
/// Returns details of a single scenario.
#[utoipa::path(
    get,
    path = "/api/v1/scenarios/{scenario_id}",
    params(
        ("scenario_id" = i32, Path, description = "Scenario ID")
    ),
    responses(
        (status = 200, description = "Scenario details", body = ScenarioResponse),
        (status = 404, description = "Scenario not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn get_scenario(
    State(state): State<AppState>,
    Path(scenario_id): Path<i32>,
) -> Result<Json<ApiResponse<ScenarioResponse>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering get_scenario function");
    debug!("Fetching scenario with id: {}", scenario_id);

    let db = &state.db;

    let scenario = scenario::Entity::find_by_id(scenario_id)
        .one(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch scenario: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?
        .ok_or_else(|| {
            warn!("Scenario not found: id={}", scenario_id);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Scenario with id {} not found", scenario_id),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    info!("Scenario fetched successfully: id={}", scenario_id);
    Ok(Json(ApiResponse {
        data: scenario.into(),
        message: "Scenario retrieved successfully".to_string(),
        success: true,
    }))
}

/// Update a scenario
///
/// Updates an existing scenario's properties.
#[utoipa::path(
    put,
    path = "/api/v1/scenarios/{scenario_id}",
    params(
        ("scenario_id" = i32, Path, description = "Scenario ID")
    ),
    request_body = UpdateScenarioRequest,
    responses(
        (status = 200, description = "Scenario updated successfully", body = ScenarioResponse),
        (status = 404, description = "Scenario not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn update_scenario(
    State(state): State<AppState>,
    Path(scenario_id): Path<i32>,
    Json(request): Json<UpdateScenarioRequest>,
) -> Result<Json<ApiResponse<ScenarioResponse>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering update_scenario function");
    debug!("Updating scenario {}: {:?}", scenario_id, request);

    let db = &state.db;

    // Fetch the existing scenario
    let scenario = scenario::Entity::find_by_id(scenario_id)
        .one(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch scenario: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?
        .ok_or_else(|| {
            warn!("Scenario not found: id={}", scenario_id);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Scenario with id {} not found", scenario_id),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    // Update the scenario
    let mut active_model: scenario::ActiveModel = scenario.into();
    if let Some(name) = request.name {
        active_model.name = Set(name);
    }
    if let Some(description) = request.description {
        active_model.description = Set(Some(description));
    }
    if let Some(is_active) = request.is_active {
        active_model.is_active = Set(is_active);
    }

    let updated = active_model.update(db).await.map_err(|e| {
        error!("Failed to update scenario: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                    error: format!("Failed to update scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
        )
    })?;

    info!("Scenario updated successfully: id={}", scenario_id);
    Ok(Json(ApiResponse {
        data: updated.into(),
        message: "Scenario updated successfully".to_string(),
        success: true,
    }))
}

/// Delete a scenario
///
/// Deletes a scenario and all its associated simulated transactions (cascade delete).
#[utoipa::path(
    delete,
    path = "/api/v1/scenarios/{scenario_id}",
    params(
        ("scenario_id" = i32, Path, description = "Scenario ID")
    ),
    responses(
        (status = 204, description = "Scenario deleted successfully"),
        (status = 404, description = "Scenario not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn delete_scenario(
    State(state): State<AppState>,
    Path(scenario_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering delete_scenario function");
    debug!("Deleting scenario: id={}", scenario_id);

    let db = &state.db;

    // Fetch the scenario to verify it exists
    let scenario = scenario::Entity::find_by_id(scenario_id)
        .one(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch scenario: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?
        .ok_or_else(|| {
            warn!("Scenario not found: id={}", scenario_id);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Scenario with id {} not found", scenario_id),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    // Delete the scenario (cascade will handle related transactions)
    let active_model: scenario::ActiveModel = scenario.into();
    active_model.delete(db).await.map_err(|e| {
        error!("Failed to delete scenario: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                    error: format!("Failed to delete scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
        )
    })?;

    info!("Scenario deleted successfully: id={}", scenario_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Apply a scenario
///
/// Applies a scenario by converting all its simulated transactions to real transactions.
/// This sets `is_simulated = false` for all transactions associated with the scenario.
#[utoipa::path(
    post,
    path = "/api/v1/scenarios/{scenario_id}/apply",
    params(
        ("scenario_id" = i32, Path, description = "Scenario ID")
    ),
    responses(
        (status = 200, description = "Scenario applied successfully", body = String),
        (status = 404, description = "Scenario not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "scenarios"
)]
#[instrument(skip(state))]
pub async fn apply_scenario(
    State(state): State<AppState>,
    Path(scenario_id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    trace!("Entering apply_scenario function");
    debug!("Applying scenario: id={}", scenario_id);

    let db = &state.db;

    // Verify scenario exists
    let scenario = scenario::Entity::find_by_id(scenario_id)
        .one(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch scenario: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch scenario: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?
        .ok_or_else(|| {
            warn!("Scenario not found: id={}", scenario_id);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Scenario with id {} not found", scenario_id),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    let mut total_applied = 0;

    // Apply one-off transactions
    let one_off_txs = one_off_transaction::Entity::find()
        .filter(one_off_transaction::Column::ScenarioId.eq(scenario_id))
        .filter(one_off_transaction::Column::IsSimulated.eq(true))
        .all(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch one-off transactions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch one-off transactions: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    for tx in one_off_txs {
        let mut active_tx: one_off_transaction::ActiveModel = tx.into();
        active_tx.is_simulated = Set(false);
        active_tx.update(db).await.map_err(|e| {
            error!("Failed to update one-off transaction: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update one-off transaction: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;
        total_applied += 1;
    }

    // Apply recurring transactions
    let recurring_txs = recurring_transaction::Entity::find()
        .filter(recurring_transaction::Column::ScenarioId.eq(scenario_id))
        .filter(recurring_transaction::Column::IsSimulated.eq(true))
        .all(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch recurring transactions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch recurring transactions: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    for tx in recurring_txs {
        let mut active_tx: recurring_transaction::ActiveModel = tx.into();
        active_tx.is_simulated = Set(false);
        active_tx.update(db).await.map_err(|e| {
            error!("Failed to update recurring transaction: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update recurring transaction: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;
        total_applied += 1;
    }

    // Apply recurring income
    let recurring_incomes = recurring_income::Entity::find()
        .filter(recurring_income::Column::ScenarioId.eq(scenario_id))
        .filter(recurring_income::Column::IsSimulated.eq(true))
        .all(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch recurring incomes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch recurring incomes: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;

    for income in recurring_incomes {
        let mut active_income: recurring_income::ActiveModel = income.into();
        active_income.is_simulated = Set(false);
        active_income.update(db).await.map_err(|e| {
            error!("Failed to update recurring income: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update recurring income: {}", e),
                    code: "SCENARIO_ERROR".to_string(),
                    success: false,
                }),
            )
        })?;
        total_applied += 1;
    }

    info!(
        "Scenario applied successfully: id={}, transactions_applied={}",
        scenario_id, total_applied
    );
    Ok(Json(ApiResponse {
        data: format!(
            "Scenario '{}' applied successfully. {} transaction(s) made real.",
            scenario.name, total_applied
        ),
        message: "Scenario applied successfully".to_string(),
        success: true,
    }))
}
