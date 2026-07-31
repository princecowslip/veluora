//! Unified cross-source search — the aggregating counterpart of
//! `POST /api/v1/search` (local-library-only). See
//! `application::discover` for the fan-out/isolation logic; this route
//! is a thin wrapper, matching `routes/search.rs`'s POST-body shape.

use application::DiscoverService;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use domain::SourceId;
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new().route("/api/v1/discover", post(discover))
}

#[derive(Deserialize)]
struct DiscoverRequest {
    query: String,
    #[serde(default)]
    source_ids: Option<Vec<String>>,
    #[serde(default = "default_limit_per_source")]
    limit_per_source: u32,
}

fn default_limit_per_source() -> u32 {
    25
}

async fn discover(State(state): State<ApiState>, Json(body): Json<DiscoverRequest>) -> Response {
    let source_ids = match body.source_ids {
        Some(raw) => match raw
            .iter()
            .map(|s| Uuid::parse_str(s).ok().map(SourceId))
            .collect::<Option<Vec<SourceId>>>()
        {
            Some(ids) => Some(ids),
            None => return bad_id_response(),
        },
        None => None,
    };
    match DiscoverService::discover(
        &state.ctx,
        &body.query,
        source_ids.as_deref(),
        body.limit_per_source,
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}
