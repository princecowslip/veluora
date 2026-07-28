use application::SearchService;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::{error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new().route("/api/v1/search", post(search))
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

fn default_limit() -> u32 {
    50
}

async fn search(
    State(state): State<ApiState>,
    Json(body): Json<SearchRequest>,
) -> impl IntoResponse {
    match SearchService::search(&state.ctx, &body.query, body.limit, body.offset) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => error_response(&e),
    }
}
