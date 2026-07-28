//! Lock-screen support for clients (like the TUI) that hold their own
//! in-memory session lock state and need to check a typed password
//! against the stored hash without embedding Argon2 themselves.

use application::PrivacyService;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::{error_response, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/privacy/status", get(privacy_status))
        .route("/api/v1/privacy/verify", post(verify_password))
}

#[derive(Serialize)]
struct PrivacyStatusResponse {
    has_password: bool,
    metadata_encryption_enabled: bool,
}

async fn privacy_status(State(state): State<ApiState>) -> Response {
    let has_password = match PrivacyService::has_password(&state.ctx) {
        Ok(v) => v,
        Err(e) => return error_response(&e),
    };
    let metadata_encryption_enabled = match PrivacyService::metadata_encryption_enabled(&state.ctx)
    {
        Ok(v) => v,
        Err(e) => return error_response(&e),
    };
    (
        StatusCode::OK,
        Json(PrivacyStatusResponse {
            has_password,
            metadata_encryption_enabled,
        }),
    )
        .into_response()
}

#[derive(Deserialize)]
struct VerifyRequest {
    password: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    ok: bool,
}

async fn verify_password(
    State(state): State<ApiState>,
    Json(body): Json<VerifyRequest>,
) -> Response {
    match PrivacyService::verify_password(&state.ctx, &body.password) {
        Ok(ok) => (StatusCode::OK, Json(VerifyResponse { ok })).into_response(),
        Err(e) => error_response(&e),
    }
}
