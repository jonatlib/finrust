use crate::schemas::{ApiResponse, AppState, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use model::entities::tag;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, DbErr, ColumnTrait, QueryFilter, RelationTrait, JoinType};
use serde::{Deserialize, Serialize};
use tracing::{instrument, error, warn, info, debug, trace};
use utoipa::ToSchema;

/// Request structure for creating a new tag
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTagRequest {
    /// The name of the tag (must be unique)
    pub name: String,
    /// Optional description of what the tag is for
    pub description: Option<String>,
    /// Optional parent tag ID for hierarchical tags
    pub parent_id: Option<i32>,
    /// Optional Ledger CLI export name template
    pub ledger_name: Option<String>,
}

/// Request structure for updating an existing tag
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTagRequest {
    /// The name of the tag (must be unique)
    pub name: Option<String>,
    /// Optional description of what the tag is for
    pub description: Option<String>,
    /// Optional parent tag ID for hierarchical tags
    pub parent_id: Option<i32>,
    /// Optional Ledger CLI export name template
    pub ledger_name: Option<String>,
}

/// Response structure for tag operations
#[derive(Debug, Serialize, ToSchema)]
pub struct TagResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub ledger_name: Option<String>,
}

impl From<tag::Model> for TagResponse {
    fn from(model: tag::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            parent_id: model.parent_id,
            ledger_name: model.ledger_name,
        }
    }
}

/// Query parameters for getting child tags
#[derive(Debug, Deserialize, ToSchema)]
pub struct TagChildrenQuery {
    /// Whether to include all nested children (true) or only direct children (false)
    pub recursive: Option<bool>,
}

/// Create a new tag
#[utoipa::path(
    post,
    path = "/api/v1/tags",
    request_body = CreateTagRequest,
    responses(
        (status = 201, description = "Tag created successfully", body = ApiResponse<TagResponse>),
        (status = 400, description = "Invalid request data", body = ErrorResponse),
        (status = 409, description = "Tag name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn create_tag(
    State(state): State<AppState>,
    Json(request): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TagResponse>>), (StatusCode, Json<ErrorResponse>)> {
    debug!("Creating tag with name: {}", request.name);

    // Validate parent_id exists if provided
    if let Some(parent_id) = request.parent_id {
        match tag::Entity::find_by_id(parent_id).one(&state.db).await {
            Ok(Some(_)) => {
                debug!("Parent tag {} exists", parent_id);
            }
            Ok(None) => {
                warn!("Parent tag {} not found", parent_id);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Parent tag with ID {} not found", parent_id),
                    }),
                ));
            }
            Err(e) => {
                error!("Database error while checking parent tag: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to validate parent tag".to_string(),
                    }),
                ));
            }
        }
    }

    let new_tag = tag::ActiveModel {
        name: Set(request.name.clone()),
        description: Set(request.description),
        parent_id: Set(request.parent_id),
        ledger_name: Set(request.ledger_name),
        ..Default::default()
    };

    match new_tag.insert(&state.db).await {
        Ok(tag_model) => {
            info!("Successfully created tag with ID: {}", tag_model.id);
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse {
                    data: TagResponse::from(tag_model),
                }),
            ))
        }
        Err(DbErr::Exec(sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(db_err))))
            if db_err.message().contains("UNIQUE constraint failed") =>
        {
            warn!("Tag name '{}' already exists", request.name);
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!("Tag with name '{}' already exists", request.name),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to create tag: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create tag".to_string(),
                }),
            ))
        }
    }
}

/// Get all tags
#[utoipa::path(
    get,
    path = "/api/v1/tags",
    responses(
        (status = 200, description = "List of all tags", body = ApiResponse<Vec<TagResponse>>),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn get_tags(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TagResponse>>>, StatusCode> {
    debug!("Fetching all tags");

    match tag::Entity::find().all(&state.db).await {
        Ok(tags) => {
            let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();
            info!("Successfully fetched {} tags", tag_responses.len());
            Ok(Json(ApiResponse { data: tag_responses }))
        }
        Err(e) => {
            error!("Failed to fetch tags: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific tag by ID
#[utoipa::path(
    get,
    path = "/api/v1/tags/{tag_id}",
    params(
        ("tag_id" = i32, Path, description = "Tag ID")
    ),
    responses(
        (status = 200, description = "Tag details", body = ApiResponse<TagResponse>),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn get_tag(
    Path(tag_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TagResponse>>, StatusCode> {
    debug!("Fetching tag with ID: {}", tag_id);

    match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(tag_model)) => {
            info!("Successfully found tag with ID: {}", tag_id);
            Ok(Json(ApiResponse {
                data: TagResponse::from(tag_model),
            }))
        }
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to fetch tag: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update an existing tag
#[utoipa::path(
    put,
    path = "/api/v1/tags/{tag_id}",
    params(
        ("tag_id" = i32, Path, description = "Tag ID")
    ),
    request_body = UpdateTagRequest,
    responses(
        (status = 200, description = "Tag updated successfully", body = ApiResponse<TagResponse>),
        (status = 400, description = "Invalid request data", body = ErrorResponse),
        (status = 404, description = "Tag not found"),
        (status = 409, description = "Tag name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn update_tag(
    Path(tag_id): Path<i32>,
    State(state): State<AppState>,
    Json(request): Json<UpdateTagRequest>,
) -> Result<Json<ApiResponse<TagResponse>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Updating tag with ID: {}", tag_id);

    // First, find the existing tag
    let existing_tag = match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Tag not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch tag: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch tag".to_string(),
                }),
            ));
        }
    };

    // Validate parent_id exists if provided and different from current tag
    if let Some(parent_id) = request.parent_id {
        if parent_id == tag_id {
            warn!("Tag cannot be its own parent");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Tag cannot be its own parent".to_string(),
                }),
            ));
        }

        match tag::Entity::find_by_id(parent_id).one(&state.db).await {
            Ok(Some(_)) => {
                debug!("Parent tag {} exists", parent_id);
            }
            Ok(None) => {
                warn!("Parent tag {} not found", parent_id);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Parent tag with ID {} not found", parent_id),
                    }),
                ));
            }
            Err(e) => {
                error!("Database error while checking parent tag: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to validate parent tag".to_string(),
                    }),
                ));
            }
        }
    }

    let mut active_tag: tag::ActiveModel = existing_tag.into();

    // Update fields if provided
    if let Some(name) = request.name {
        active_tag.name = Set(name);
    }
    if let Some(description) = request.description {
        active_tag.description = Set(Some(description));
    }
    if let Some(parent_id) = request.parent_id {
        active_tag.parent_id = Set(Some(parent_id));
    }
    if let Some(ledger_name) = request.ledger_name {
        active_tag.ledger_name = Set(Some(ledger_name));
    }

    match active_tag.update(&state.db).await {
        Ok(updated_tag) => {
            info!("Successfully updated tag with ID: {}", tag_id);
            Ok(Json(ApiResponse {
                data: TagResponse::from(updated_tag),
            }))
        }
        Err(DbErr::Exec(sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(db_err))))
            if db_err.message().contains("UNIQUE constraint failed") =>
        {
            warn!("Tag name already exists");
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Tag with this name already exists".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to update tag: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update tag".to_string(),
                }),
            ))
        }
    }
}

/// Delete a tag
#[utoipa::path(
    delete,
    path = "/api/v1/tags/{tag_id}",
    params(
        ("tag_id" = i32, Path, description = "Tag ID")
    ),
    responses(
        (status = 200, description = "Tag deleted successfully", body = ApiResponse<String>),
        (status = 400, description = "Cannot delete tag with children", body = ErrorResponse),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn delete_tag(
    Path(tag_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Deleting tag with ID: {}", tag_id);

    // Check if tag exists
    let existing_tag = match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Tag not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch tag: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch tag".to_string(),
                }),
            ));
        }
    };

    // Check if tag has children
    let children_count = match tag::Entity::find()
        .filter(tag::Column::ParentId.eq(tag_id))
        .count(&state.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to count child tags: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check for child tags".to_string(),
                }),
            ));
        }
    };

    if children_count > 0 {
        warn!("Cannot delete tag {} as it has {} children", tag_id, children_count);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Cannot delete tag as it has {} child tags. Delete or move child tags first.", children_count),
            }),
        ));
    }

    match tag::Entity::delete_by_id(tag_id).exec(&state.db).await {
        Ok(delete_result) => {
            if delete_result.rows_affected > 0 {
                info!("Successfully deleted tag with ID: {}", tag_id);
                Ok(Json(ApiResponse {
                    data: format!("Tag with ID {} deleted successfully", tag_id),
                }))
            } else {
                warn!("No tag was deleted with ID: {}", tag_id);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Tag not found".to_string(),
                    }),
                ))
            }
        }
        Err(e) => {
            error!("Failed to delete tag: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete tag".to_string(),
                }),
            ))
        }
    }
}

/// Get child tags of a specific tag
#[utoipa::path(
    get,
    path = "/api/v1/tags/{tag_id}/children",
    params(
        ("tag_id" = i32, Path, description = "Tag ID"),
        ("recursive" = Option<bool>, Query, description = "Include all nested children (true) or only direct children (false)")
    ),
    responses(
        (status = 200, description = "List of child tags", body = ApiResponse<Vec<TagResponse>>),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn get_tag_children(
    Path(tag_id): Path<i32>,
    Query(query): Query<TagChildrenQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TagResponse>>>, StatusCode> {
    debug!("Fetching children for tag ID: {}, recursive: {:?}", tag_id, query.recursive);

    // Check if parent tag exists
    match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to fetch parent tag: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let recursive = query.recursive.unwrap_or(false);

    if recursive {
        // Get all nested children recursively
        match get_all_nested_children(&state.db, tag_id).await {
            Ok(children) => {
                let tag_responses: Vec<TagResponse> = children.into_iter().map(TagResponse::from).collect();
                info!("Successfully fetched {} nested children for tag {}", tag_responses.len(), tag_id);
                Ok(Json(ApiResponse { data: tag_responses }))
            }
            Err(e) => {
                error!("Failed to fetch nested children: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // Get only direct children
        match tag::Entity::find()
            .filter(tag::Column::ParentId.eq(tag_id))
            .all(&state.db)
            .await
        {
            Ok(children) => {
                let tag_responses: Vec<TagResponse> = children.into_iter().map(TagResponse::from).collect();
                info!("Successfully fetched {} direct children for tag {}", tag_responses.len(), tag_id);
                Ok(Json(ApiResponse { data: tag_responses }))
            }
            Err(e) => {
                error!("Failed to fetch direct children: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Link a tag to a parent (move tag under another tag)
#[utoipa::path(
    put,
    path = "/api/v1/tags/{tag_id}/parent/{parent_id}",
    params(
        ("tag_id" = i32, Path, description = "Tag ID to move"),
        ("parent_id" = i32, Path, description = "New parent tag ID")
    ),
    responses(
        (status = 200, description = "Tag linked successfully", body = ApiResponse<TagResponse>),
        (status = 400, description = "Invalid operation (circular reference)", body = ErrorResponse),
        (status = 404, description = "Tag or parent not found", body = ErrorResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn link_tag_to_parent(
    Path((tag_id, parent_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TagResponse>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Linking tag {} to parent {}", tag_id, parent_id);

    if tag_id == parent_id {
        warn!("Tag cannot be its own parent");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tag cannot be its own parent".to_string(),
            }),
        ));
    }

    // Check if both tags exist
    let existing_tag = match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Tag not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch tag: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch tag".to_string(),
                }),
            ));
        }
    };

    match tag::Entity::find_by_id(parent_id).one(&state.db).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!("Parent tag with ID {} not found", parent_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Parent tag not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch parent tag: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch parent tag".to_string(),
                }),
            ));
        }
    }

    // Check for circular reference (parent_id should not be a descendant of tag_id)
    match is_descendant(&state.db, parent_id, tag_id).await {
        Ok(true) => {
            warn!("Cannot link tag {} to parent {} - would create circular reference", tag_id, parent_id);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Cannot create circular reference in tag hierarchy".to_string(),
                }),
            ));
        }
        Ok(false) => {}
        Err(e) => {
            error!("Failed to check for circular reference: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to validate tag hierarchy".to_string(),
                }),
            ));
        }
    }

    let mut active_tag: tag::ActiveModel = existing_tag.into();
    active_tag.parent_id = Set(Some(parent_id));

    match active_tag.update(&state.db).await {
        Ok(updated_tag) => {
            info!("Successfully linked tag {} to parent {}", tag_id, parent_id);
            Ok(Json(ApiResponse {
                data: TagResponse::from(updated_tag),
            }))
        }
        Err(e) => {
            error!("Failed to link tag to parent: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to link tag to parent".to_string(),
                }),
            ))
        }
    }
}

/// Unlink a tag from its parent (make it a root tag)
#[utoipa::path(
    delete,
    path = "/api/v1/tags/{tag_id}/parent",
    params(
        ("tag_id" = i32, Path, description = "Tag ID to unlink")
    ),
    responses(
        (status = 200, description = "Tag unlinked successfully", body = ApiResponse<TagResponse>),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "tags"
)]
#[instrument(skip(state))]
pub async fn unlink_tag_from_parent(
    Path(tag_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TagResponse>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Unlinking tag {} from its parent", tag_id);

    let existing_tag = match tag::Entity::find_by_id(tag_id).one(&state.db).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            warn!("Tag with ID {} not found", tag_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Tag not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to fetch tag: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch tag".to_string(),
                }),
            ));
        }
    };

    let mut active_tag: tag::ActiveModel = existing_tag.into();
    active_tag.parent_id = Set(None);

    match active_tag.update(&state.db).await {
        Ok(updated_tag) => {
            info!("Successfully unlinked tag {} from its parent", tag_id);
            Ok(Json(ApiResponse {
                data: TagResponse::from(updated_tag),
            }))
        }
        Err(e) => {
            error!("Failed to unlink tag from parent: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to unlink tag from parent".to_string(),
                }),
            ))
        }
    }
}

// Helper functions

/// Recursively get all nested children of a tag
async fn get_all_nested_children(db: &sea_orm::DatabaseConnection, parent_id: i32) -> Result<Vec<tag::Model>, DbErr> {
    let mut all_children = Vec::new();
    let mut to_process = vec![parent_id];

    while let Some(current_parent_id) = to_process.pop() {
        let direct_children = tag::Entity::find()
            .filter(tag::Column::ParentId.eq(current_parent_id))
            .all(db)
            .await?;

        for child in direct_children {
            to_process.push(child.id);
            all_children.push(child);
        }
    }

    Ok(all_children)
}

/// Check if a tag is a descendant of another tag
async fn is_descendant(db: &sea_orm::DatabaseConnection, potential_descendant: i32, ancestor: i32) -> Result<bool, DbErr> {
    let mut current_id = potential_descendant;

    loop {
        match tag::Entity::find_by_id(current_id).one(db).await? {
            Some(tag) => {
                if let Some(parent_id) = tag.parent_id {
                    if parent_id == ancestor {
                        return Ok(true);
                    }
                    current_id = parent_id;
                } else {
                    // Reached root, no circular reference
                    return Ok(false);
                }
            }
            None => {
                // Tag not found, no circular reference
                return Ok(false);
            }
        }
    }
}