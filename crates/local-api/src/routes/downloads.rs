//! Downloads and offline use (Workstream 11) routes — see
//! `application::download::DownloadService` for the underlying logic.
//!
//! `local-api` is the only long-lived process among this codebase's
//! surfaces (the CLI is one-shot), so it's the one that can genuinely
//! run a download to completion in the background: `add`/`resume`
//! spawn `DownloadService::run` on the server's already-running
//! multi-threaded runtime and return `202 Accepted` immediately —
//! callers poll `GET /api/v1/downloads/:id` for progress.

use std::sync::Arc;

use application::{AppContext, DownloadService, PrivacyService, SettingsService};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::{DownloadId, ItemId, VariantId};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{bad_id_response, error_response, ApiState};

/// Spawns `DownloadService::run` gated by `semaphore` — an `add`/`resume`
/// call beyond `max_concurrent_downloads` just waits on the permit
/// (the row stays `Queued`/`Paused` in the meantime), so the semaphore
/// itself is the whole scheduler; no separate queue data structure is
/// needed.
pub(crate) fn spawn_download(ctx: &Arc<AppContext>, semaphore: &Arc<Semaphore>, id: DownloadId) {
    let ctx = ctx.clone();
    let semaphore = semaphore.clone();
    tokio::spawn(async move {
        let Ok(_permit) = semaphore.acquire_owned().await else {
            return;
        };
        let _ = DownloadService::run(&ctx, id).await;
    });
}

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/downloads", get(list).post(add))
        .route("/api/v1/downloads/status", get(status))
        .route("/api/v1/downloads/quota", post(set_quota))
        .route("/api/v1/downloads/enforce-quota", post(enforce_quota))
        .route("/api/v1/downloads/eligibility", get(eligibility))
        .route("/api/v1/downloads/:id", get(find).delete(remove))
        .route("/api/v1/downloads/:id/pause", post(pause))
        .route("/api/v1/downloads/:id/resume", post(resume))
        .route("/api/v1/downloads/:id/cancel", post(cancel))
        .route("/api/v1/downloads/:id/pin", post(set_pinned))
}

#[derive(Deserialize)]
struct ListQuery {
    item_id: Option<String>,
}

async fn list(State(state): State<ApiState>, Query(params): Query<ListQuery>) -> Response {
    let item_id = match params.item_id {
        Some(raw) => match parse_item_id(&raw) {
            Some(id) => Some(id),
            None => return bad_id_response(),
        },
        None => None,
    };
    match DownloadService::list(&state.ctx, item_id) {
        Ok(downloads) => (StatusCode::OK, Json(downloads)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct AddDownloadRequest {
    item_id: String,
    variant_id: String,
}

async fn add(State(state): State<ApiState>, Json(body): Json<AddDownloadRequest>) -> Response {
    let Some(item_id) = parse_item_id(&body.item_id) else {
        return bad_id_response();
    };
    let Some(variant_id) = parse_variant_id(&body.variant_id) else {
        return bad_id_response();
    };
    match DownloadService::add(&state.ctx, item_id, variant_id) {
        Ok(download) => {
            spawn_download(&state.ctx, &state.download_semaphore, download.id);
            (StatusCode::ACCEPTED, Json(download)).into_response()
        }
        Err(e) => error_response(&e),
    }
}

async fn find(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::find(&state.ctx, download_id) {
        Ok(Some(download)) => (StatusCode::OK, Json(download)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "schema_version": 1, "error": "not found" })),
        )
            .into_response(),
        Err(e) => error_response(&e),
    }
}

async fn pause(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::pause(&state.ctx, download_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn resume(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::find(&state.ctx, download_id) {
        Ok(Some(download)) => {
            spawn_download(&state.ctx, &state.download_semaphore, download_id);
            (StatusCode::ACCEPTED, Json(download)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "schema_version": 1, "error": "not found" })),
        )
            .into_response(),
        Err(e) => error_response(&e),
    }
}

async fn cancel(State(state): State<ApiState>, AxumPath(id): AxumPath<String>) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::cancel(&state.ctx, download_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct PinRequest {
    pinned: bool,
}

async fn set_pinned(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<PinRequest>,
) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::set_pinned(&state.ctx, download_id, body.pinned) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct RemoveQuery {
    #[serde(default)]
    delete_file: bool,
}

async fn remove(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<RemoveQuery>,
) -> Response {
    let Some(download_id) = parse_download_id(&id) else {
        return bad_id_response();
    };
    match DownloadService::remove(&state.ctx, download_id, params.delete_file) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct EligibilityQuery {
    item_id: String,
    variant_id: String,
}

async fn eligibility(
    State(state): State<ApiState>,
    Query(params): Query<EligibilityQuery>,
) -> Response {
    let Some(item_id) = parse_item_id(&params.item_id) else {
        return bad_id_response();
    };
    let Some(variant_id) = parse_variant_id(&params.variant_id) else {
        return bad_id_response();
    };
    match DownloadService::check_eligibility(&state.ctx, item_id, variant_id) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Serialize)]
struct DownloadStatusResponse {
    total_bytes: u64,
    quota_bytes: Option<u64>,
}

async fn status(State(state): State<ApiState>) -> Response {
    let total_bytes = match PrivacyService::download_directory_size_bytes(&state.ctx) {
        Ok(b) => b,
        Err(e) => return error_response(&e),
    };
    let quota_bytes = match SettingsService::download_quota_bytes(&state.ctx) {
        Ok(q) => q,
        Err(e) => return error_response(&e),
    };
    (
        StatusCode::OK,
        Json(DownloadStatusResponse {
            total_bytes,
            quota_bytes,
        }),
    )
        .into_response()
}

#[derive(Deserialize)]
struct SetQuotaRequest {
    bytes: Option<u64>,
}

async fn set_quota(State(state): State<ApiState>, Json(body): Json<SetQuotaRequest>) -> Response {
    match SettingsService::set_download_quota_bytes(&state.ctx, body.bytes) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(&e),
    }
}

async fn enforce_quota(State(state): State<ApiState>) -> Response {
    match PrivacyService::enforce_download_quota(&state.ctx) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => error_response(&e),
    }
}

fn parse_download_id(raw: &str) -> Option<DownloadId> {
    Uuid::parse_str(raw).ok().map(DownloadId)
}

fn parse_item_id(raw: &str) -> Option<ItemId> {
    Uuid::parse_str(raw).ok().map(ItemId)
}

fn parse_variant_id(raw: &str) -> Option<VariantId> {
    Uuid::parse_str(raw).ok().map(VariantId)
}
