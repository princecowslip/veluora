//! Loopback-only local API, per `docs/19-local-api.md`.
//!
//! Binding is always `127.0.0.1` — there is no configuration path to a
//! non-loopback address; remote access is explicitly a separate, later
//! feature per the doc. Every route is bearer-token authenticated except
//! `GET /health`, enforced once via [`require_auth`] middleware rather
//! than a per-handler check, now that Milestone B adds a dozen protected
//! routes on top of Milestone A's single `/diagnostics/summary`.

mod routes;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use application::{AppContext, AppError, DiagnosticsService, DownloadService, SettingsService};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use domain::DownloadId;
use rand::Rng;
use serde_json::json;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct ApiState {
    pub ctx: Arc<AppContext>,
    pub token: Arc<str>,
    pub download_semaphore: Arc<Semaphore>,
}

impl ApiState {
    /// Builds state with a semaphore sized from
    /// [`SettingsService::max_concurrent_downloads`]. The one non-test
    /// constructor — use this from `main.rs` rather than the struct
    /// literal, so the cap is never accidentally left unbounded.
    pub fn new(ctx: Arc<AppContext>, token: Arc<str>) -> application::Result<Self> {
        let cap = SettingsService::max_concurrent_downloads(&ctx)?.max(1) as usize;
        Ok(Self {
            ctx,
            token,
            download_semaphore: Arc::new(Semaphore::new(cap)),
        })
    }
}

/// Runs startup download recovery: repairs any `Active` row a prior
/// instance left stuck (see `application::download::DownloadService::repair_stale_active`
/// for why this is time-based rather than an unconditional sweep — a
/// sibling GUI process can legitimately be running a download against
/// the same database at the same time), sweeps now-truly-orphaned temp
/// files, and re-launches every `Queued`/crash-recovered-`Paused` row
/// through the same semaphore-gated path `add`/`resume` use. Called
/// once at process startup and again from a periodic background task,
/// so a row that goes stale *while this process is running* also
/// eventually recovers.
pub fn recover_and_resume_downloads(
    ctx: &Arc<AppContext>,
    download_semaphore: &Arc<Semaphore>,
) -> application::Result<Vec<DownloadId>> {
    DownloadService::repair_stale_active(ctx, DownloadService::DEFAULT_STALE_ACTIVE_THRESHOLD)?;
    let _ = DownloadService::sweep_orphaned_temp_files(ctx);
    let resumable = DownloadService::resumable_after_restart(ctx)?;
    for id in &resumable {
        routes::downloads::spawn_download(ctx, download_semaphore, *id);
    }
    Ok(resumable)
}

pub fn build_router(state: ApiState) -> Router {
    let protected = Router::new()
        .route("/api/v1/diagnostics/summary", get(diagnostics_summary))
        .merge(routes::block_rules::router())
        .merge(routes::library::router())
        .merge(routes::search::router())
        .merge(routes::discover::router())
        .merge(routes::items::router())
        .merge(routes::collections::router())
        .merge(routes::cache::router())
        .merge(routes::home::router())
        .merge(routes::privacy::router())
        .merge(routes::sources::router())
        .merge(routes::downloads::router())
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(protected)
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "schema_version": 1, "status": "ok" }))
}

async fn diagnostics_summary(State(state): State<ApiState>) -> impl IntoResponse {
    match DiagnosticsService::summary(&state.ctx) {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(err) => error_response(&err),
    }
}

async fn require_auth(
    State(state): State<ApiState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if is_authorized(&headers, &state.token) {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "schema_version": 1, "error": "authentication required" })),
        )
            .into_response()
    }
}

fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|presented| presented == expected_token)
}

/// Maps an `AppError` to a status code and a `{schema_version, error}`
/// body, used by every route handler under `routes/`.
pub(crate) fn error_response(err: &AppError) -> Response {
    let status = match err {
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::InvalidQuery(_) | AppError::InvalidPath(_) => StatusCode::BAD_REQUEST,
        AppError::UnsupportedCapability(_) => StatusCode::UNPROCESSABLE_ENTITY,
        AppError::Network(_) => StatusCode::BAD_GATEWAY,
        AppError::Database(_) | AppError::Io(_) | AppError::NoDataDirectory => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(json!({ "schema_version": 1, "error": err.to_string() })),
    )
        .into_response()
}

pub(crate) fn bad_id_response() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "schema_version": 1, "error": "invalid id" })),
    )
        .into_response()
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

pub fn port_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("api-port")
}

/// Writes the bound port to `<data_dir>/api-port`, mirroring
/// [`write_token_file`]'s discovery convention. Unlike the token, the
/// port isn't a secret — this is what lets the TUI (a separate process
/// per `docs/12-system-architecture.md`'s notcurses TUI boundary, which
/// cannot link this crate in-process) find the loopback API without a
/// fixed, hardcoded port number.
pub fn write_port_file(data_dir: &Path, port: u16) -> std::io::Result<PathBuf> {
    fs::create_dir_all(data_dir)?;
    let path = port_file_path(data_dir);
    fs::write(&path, port.to_string())?;
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

    #[test]
    fn write_port_file_writes_the_plain_port_number() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_port_file(dir.path(), 54321).unwrap();
        assert_eq!(path, port_file_path(dir.path()));
        assert_eq!(fs::read_to_string(path).unwrap(), "54321");
    }

    #[tokio::test]
    async fn health_is_reachable_without_a_token() {
        let ctx = Arc::new(AppContext::open_in_memory().expect("context"));
        let state = ApiState::new(ctx, Arc::from(generate_token())).unwrap();
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
        let state = ApiState::new(ctx, Arc::from(token.clone())).unwrap();

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
