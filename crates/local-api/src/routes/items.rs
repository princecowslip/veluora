use application::{ItemService, UserStateService};
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::ItemId;
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/items/:id", get(get_item))
        .route("/api/v1/items/:id/favorite", post(set_favorite))
}

async fn get_item(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match ItemService::get(&state.ctx, item_id) {
        Ok(detail) => (StatusCode::OK, Json(detail)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct FavoriteRequest {
    favorite: bool,
}

async fn set_favorite(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<FavoriteRequest>,
) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match UserStateService::set_favorite(&state.ctx, item_id, body.favorite) {
        Ok(user_state) => (StatusCode::OK, Json(user_state)).into_response(),
        Err(e) => error_response(&e),
    }
}

fn parse_item_id(raw: &str) -> Option<ItemId> {
    Uuid::parse_str(raw).ok().map(ItemId)
}
