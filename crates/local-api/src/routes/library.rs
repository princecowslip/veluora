use std::path::PathBuf;
use std::sync::Arc;

use application::{LibraryRootService, LibraryService, ScanService};
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use domain::LibraryRootId;
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/library/roots", get(list_roots).post(add_root))
        .route("/api/v1/library/roots/:id", delete(remove_root))
        .route("/api/v1/library/scan", post(scan))
        .route("/api/v1/library/status", get(status))
}

async fn list_roots(State(state): State<ApiState>) -> impl IntoResponse {
    match LibraryRootService::list(&state.ctx) {
        Ok(roots) => (StatusCode::OK, Json(roots)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct AddRootRequest {
    path: String,
    display_name: Option<String>,
}

async fn add_root(
    State(state): State<ApiState>,
    Json(body): Json<AddRootRequest>,
) -> impl IntoResponse {
    match LibraryRootService::add(&state.ctx, &PathBuf::from(body.path), body.display_name) {
        Ok(root) => (StatusCode::CREATED, Json(root)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn remove_root(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return bad_id_response();
    };
    match LibraryRootService::remove(&state.ctx, LibraryRootId(uuid)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize, Default)]
struct ScanRequest {
    #[serde(default)]
    root_id: Option<String>,
}

/// Runs the (synchronous, file-I/O-heavy) scan via `spawn_blocking` so it
/// doesn't stall the async runtime. The HTTP response still blocks until
/// the scan finishes — streaming progress is a documented fast-follow,
/// not built here.
async fn scan(State(state): State<ApiState>, Json(body): Json<ScanRequest>) -> impl IntoResponse {
    let ctx = Arc::clone(&state.ctx);
    let root_id = body.root_id;

    let result = tokio::task::spawn_blocking(move || match root_id {
        Some(id_str) => match Uuid::parse_str(&id_str) {
            Ok(uuid) => ScanService::scan_root(&ctx, LibraryRootId(uuid)),
            Err(_) => Err(application::AppError::InvalidPath(
                "invalid root_id".to_string(),
            )),
        },
        None => ScanService::scan_all(&ctx),
    })
    .await
    .expect("scan task panicked");

    match result {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn status(State(state): State<ApiState>) -> impl IntoResponse {
    match LibraryService::status(&state.ctx) {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => error_response(&e),
    }
}
