//! Cache breakdown/quota routes — the local-only reinterpretation of
//! Milestone G's "downloads/offline" half. See
//! `application::privacy::PrivacyService` for the underlying logic.

use application::{CacheBreakdown, PrivacyService, SettingsService};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::{error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/cache/status", get(cache_status))
        .route("/api/v1/cache/quota", post(set_cache_quota))
        .route("/api/v1/cache/enforce-quota", post(enforce_cache_quota))
}

#[derive(Serialize)]
struct CacheStatusResponse {
    breakdown: CacheBreakdown,
    quota_bytes: Option<u64>,
}

async fn cache_status(State(state): State<ApiState>) -> Response {
    let breakdown = match PrivacyService::cache_breakdown(&state.ctx) {
        Ok(b) => b,
        Err(e) => return error_response(&e),
    };
    let quota_bytes = match SettingsService::cache_quota_bytes(&state.ctx) {
        Ok(q) => q,
        Err(e) => return error_response(&e),
    };
    (
        StatusCode::OK,
        Json(CacheStatusResponse {
            breakdown,
            quota_bytes,
        }),
    )
        .into_response()
}

#[derive(Deserialize)]
struct SetQuotaRequest {
    bytes: Option<u64>,
}

async fn set_cache_quota(
    State(state): State<ApiState>,
    Json(body): Json<SetQuotaRequest>,
) -> Response {
    match SettingsService::set_cache_quota_bytes(&state.ctx, body.bytes) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn enforce_cache_quota(State(state): State<ApiState>) -> Response {
    match PrivacyService::enforce_cache_quota(&state.ctx) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}
