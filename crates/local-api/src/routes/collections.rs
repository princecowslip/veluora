use application::CollectionService;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use domain::{CollectionId, ItemId};
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/collections", get(list).post(create))
        .route("/api/v1/collections/:id", delete(remove))
        .route("/api/v1/collections/:id/items", post(add_item))
        .route(
            "/api/v1/collections/:id/items/:item_id",
            delete(remove_item),
        )
}

async fn list(State(state): State<ApiState>) -> Response {
    match CollectionService::list(&state.ctx) {
        Ok(collections) => (StatusCode::OK, Json(collections)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct CreateCollectionRequest {
    name: String,
    description: Option<String>,
}

async fn create(
    State(state): State<ApiState>,
    Json(body): Json<CreateCollectionRequest>,
) -> Response {
    match CollectionService::create(&state.ctx, &body.name, body.description.as_deref()) {
        Ok(collection) => (StatusCode::CREATED, Json(collection)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn remove(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(collection_id) = parse_collection_id(&id) else {
        return bad_id_response();
    };
    match CollectionService::delete(&state.ctx, collection_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct AddItemRequest {
    item_id: String,
}

async fn add_item(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<AddItemRequest>,
) -> Response {
    let (Some(collection_id), Some(item_id)) =
        (parse_collection_id(&id), parse_item_id(&body.item_id))
    else {
        return bad_id_response();
    };
    match CollectionService::add_item(&state.ctx, collection_id, item_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn remove_item(
    State(state): State<ApiState>,
    AxumPath((id, item_id)): AxumPath<(String, String)>,
) -> Response {
    let (Some(collection_id), Some(item_id)) = (parse_collection_id(&id), parse_item_id(&item_id))
    else {
        return bad_id_response();
    };
    match CollectionService::remove_item(&state.ctx, collection_id, item_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

fn parse_collection_id(raw: &str) -> Option<CollectionId> {
    Uuid::parse_str(raw).ok().map(CollectionId)
}

fn parse_item_id(raw: &str) -> Option<ItemId> {
    Uuid::parse_str(raw).ok().map(ItemId)
}
