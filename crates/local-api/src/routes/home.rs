//! The Home view's "Continue" list — wraps `SearchService::continue_items`,
//! which existed in `application` without an HTTP surface until the TUI
//! (Milestone G) needed one.

use application::SearchService;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::{error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new().route("/api/v1/home/continue", get(continue_items))
}

#[derive(Deserialize)]
struct ContinueQuery {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    20
}

async fn continue_items(
    State(state): State<ApiState>,
    Query(query): Query<ContinueQuery>,
) -> Response {
    match SearchService::continue_items(&state.ctx, query.limit) {
        Ok(items) => (StatusCode::OK, Json(items)).into_response(),
        Err(e) => error_response(&e),
    }
}
