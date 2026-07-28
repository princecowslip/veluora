use application::{ComicService, ItemService, PlaybackService, StoryService, UserStateService};
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::{ItemId, Progress, StoryFormat};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/items/:id", get(get_item))
        .route("/api/v1/items/:id/favorite", post(set_favorite))
        .route("/api/v1/items/:id/open", post(open_item))
        .route("/api/v1/items/:id/progress", post(set_progress))
        .route("/api/v1/items/:id/story", get(get_story))
        .route("/api/v1/items/:id/pages", get(list_pages))
        .route("/api/v1/items/:id/pages/:index", get(get_page))
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

/// Resolves what opening the item means — it never spawns a process
/// itself; that stays a CLI/future-GUI concern (see `application::playback`).
async fn open_item(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match PlaybackService::resolve_open(&state.ctx, item_id) {
        Ok(target) => (StatusCode::OK, Json(target)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// Flattens `domain::Progress`'s internally-tagged JSON shape
/// (`{"progress_type": "...", ...}`) alongside a sibling `completed`
/// override field.
#[derive(Deserialize)]
struct ProgressRequest {
    #[serde(flatten)]
    progress: Progress,
    #[serde(default)]
    completed: Option<bool>,
}

async fn set_progress(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<ProgressRequest>,
) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match PlaybackService::record_progress(&state.ctx, item_id, body.progress, body.completed) {
        Ok(user_state) => (StatusCode::OK, Json(user_state)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Serialize)]
struct StoryResponse {
    format: StoryFormat,
    content: String,
    chapter_map: serde_json::Value,
}

async fn get_story(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    let doc = match StoryService::get(&state.ctx, item_id) {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return error_response(&application::AppError::NotFound(format!(
                "story document for item {item_id}"
            )))
        }
        Err(e) => return error_response(&e),
    };
    match StoryService::read_content(&state.ctx, item_id) {
        Ok(content) => (
            StatusCode::OK,
            Json(StoryResponse {
                format: doc.format,
                content,
                chapter_map: doc.chapter_map,
            }),
        )
            .into_response(),
        Err(e) => error_response(&e),
    }
}

async fn list_pages(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match ComicService::pages(&state.ctx, item_id) {
        Ok(pages) => (StatusCode::OK, Json(pages)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn get_page(
    State(state): State<ApiState>,
    AxumPath((id, index)): AxumPath<(String, u32)>,
) -> Response {
    let Some(item_id) = parse_item_id(&id) else {
        return bad_id_response();
    };
    match ComicService::page_bytes(&state.ctx, item_id, index) {
        Ok((bytes, mime)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(&mime)
                    .unwrap_or(HeaderValue::from_static("application/octet-stream")),
            );
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(e) => error_response(&e),
    }
}

fn parse_item_id(raw: &str) -> Option<ItemId> {
    Uuid::parse_str(raw).ok().map(ItemId)
}
