use application::BlockRuleService;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use domain::{BlockRuleId, RuleType, Scope};
use serde::Deserialize;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/block-rules", get(list).post(add))
        .route("/api/v1/block-rules/:id", delete(remove))
        .route("/api/v1/block-rules/:id/enable", post(enable))
        .route("/api/v1/block-rules/:id/disable", post(disable))
}

async fn list(State(state): State<ApiState>) -> Response {
    match BlockRuleService::list(&state.ctx) {
        Ok(rules) => (StatusCode::OK, Json(rules)).into_response(),
        Err(e) => error_response(&e),
    }
}

fn default_scope() -> Scope {
    Scope::All
}

#[derive(Deserialize)]
struct AddBlockRuleRequest {
    rule_type: RuleType,
    target: String,
    #[serde(default = "default_scope")]
    scope: Scope,
    #[serde(default)]
    reason: Option<String>,
}

async fn add(State(state): State<ApiState>, Json(body): Json<AddBlockRuleRequest>) -> Response {
    match BlockRuleService::create(
        &state.ctx,
        body.rule_type,
        body.target,
        body.scope,
        body.reason,
    ) {
        Ok(rule) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn remove(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(block_rule_id) = parse_block_rule_id(&id) else {
        return bad_id_response();
    };
    match BlockRuleService::remove(&state.ctx, block_rule_id) {
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
    let Some(block_rule_id) = parse_block_rule_id(&id) else {
        return bad_id_response();
    };
    match BlockRuleService::set_enabled(&state.ctx, block_rule_id, enabled) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

fn parse_block_rule_id(raw: &str) -> Option<BlockRuleId> {
    Uuid::parse_str(raw).ok().map(BlockRuleId)
}
