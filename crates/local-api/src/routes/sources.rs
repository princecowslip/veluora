use application::SourceService;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use domain::{ConnectorId, RemoteItem, SourceId};
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/sources", get(list).post(add))
        .route("/api/v1/sources/:id", delete(remove))
        .route("/api/v1/sources/:id/enable", post(enable))
        .route("/api/v1/sources/:id/disable", post(disable))
        .route("/api/v1/sources/:id/health-check", post(health_check))
        .route("/api/v1/sources/:id/browse", get(browse))
        .route("/api/v1/sources/:id/import", post(import))
}

async fn list(State(state): State<ApiState>) -> Response {
    match SourceService::list(&state.ctx) {
        Ok(sources) => (StatusCode::OK, Json(sources)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct AddSourceRequest {
    connector_id: String,
    display_name: String,
    #[serde(default)]
    configuration_json: serde_json::Value,
}

async fn add(State(state): State<ApiState>, Json(body): Json<AddSourceRequest>) -> Response {
    let Some(connector_id) = parse_connector_id(&body.connector_id) else {
        return bad_id_response();
    };
    match SourceService::add(
        &state.ctx,
        connector_id,
        body.display_name,
        body.configuration_json,
    ) {
        Ok(source) => (StatusCode::CREATED, Json(source)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn remove(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(source_id) = parse_source_id(&id) else {
        return bad_id_response();
    };
    match SourceService::remove(&state.ctx, source_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn enable(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    set_enabled(state, id, true).await
}

async fn disable(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    set_enabled(state, id, false).await
}

async fn set_enabled(state: ApiState, id: String, enabled: bool) -> Response {
    let Some(source_id) = parse_source_id(&id) else {
        return bad_id_response();
    };
    match SourceService::set_enabled(&state.ctx, source_id, enabled) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn health_check(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(source_id) = parse_source_id(&id) else {
        return bad_id_response();
    };
    match SourceService::health_check(&state.ctx, source_id).await {
        Ok(health) => (StatusCode::OK, Json(health)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct BrowseQuery {
    query: Option<String>,
}

async fn browse(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<BrowseQuery>,
) -> Response {
    let Some(source_id) = parse_source_id(&id) else {
        return bad_id_response();
    };
    match SourceService::browse(&state.ctx, source_id, params.query.as_deref()).await {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn import(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Json(remote_item): Json<RemoteItem>,
) -> Response {
    let Some(source_id) = parse_source_id(&id) else {
        return bad_id_response();
    };
    match SourceService::import_remote_item(&state.ctx, source_id, remote_item) {
        Ok(item_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "item_id": item_id.to_string() })),
        )
            .into_response(),
        Err(e) => error_response(&e),
    }
}

fn parse_source_id(raw: &str) -> Option<SourceId> {
    Uuid::parse_str(raw).ok().map(SourceId)
}

fn parse_connector_id(raw: &str) -> Option<ConnectorId> {
    Uuid::parse_str(raw).ok().map(ConnectorId)
}
