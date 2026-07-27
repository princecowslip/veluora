//! Loopback-only local API skeleton, per `docs/19-local-api.md`.
//!
//! Milestone A implements the "Health" endpoint subset only:
//! `GET /health` (unauthenticated liveness) and
//! `GET /diagnostics/summary` (bearer-token authenticated). Binding is
//! always `127.0.0.1` — there is no configuration path to a non-loopback
//! address in this milestone; remote access is explicitly a separate,
//! later feature per the doc.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use application::{AppContext, DiagnosticsService};
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use rand::Rng;
use serde_json::json;

#[derive(Clone)]
pub struct ApiState {
    pub ctx: Arc<AppContext>,
    pub token: Arc<str>,
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/diagnostics/summary", get(diagnostics_summary))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "schema_version": 1, "status": "ok" }))
}

async fn diagnostics_summary(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_authorized(&headers, &state.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "schema_version": 1, "error": "authentication required" })),
        )
            .into_response();
    }

    match DiagnosticsService::summary(&state.ctx) {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "schema_version": 1, "error": err.to_string() })),
        )
            .into_response(),
    }
}

fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|presented| presented == expected_token)
}

/// A random 256-bit bearer token, hex-encoded.
pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn token_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("api-token")
}

/// Writes the token to `<data_dir>/api-token` with owner-only permissions
/// on Unix. Per `docs/19-local-api.md`: never log or print the token
/// itself, only the location it was written to.
pub fn write_token_file(data_dir: &Path, token: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(data_dir)?;
    let path = token_file_path(data_dir);
    fs::write(&path, token)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt as _;

    #[test]
    fn generated_tokens_are_64_hex_chars_and_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn health_is_reachable_without_a_token() {
        let ctx = Arc::new(AppContext::open_in_memory().expect("context"));
        let state = ApiState {
            ctx,
            token: Arc::from(generate_token()),
        };
        let app = build_router(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn diagnostics_summary_requires_a_valid_token() {
        let ctx = Arc::new(AppContext::open_in_memory().expect("context"));
        let token = generate_token();
        let state = ApiState {
            ctx,
            token: Arc::from(token.clone()),
        };

        let unauthorized = build_router(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/diagnostics/summary")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let authorized = build_router(state)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/diagnostics/summary")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(authorized.status(), StatusCode::OK);
    }
}
