//! End-to-end `DownloadService` coverage against real local HTTP
//! fixture servers — same approach as `crates/connectors/tests/feed_connector.rs`
//! and `crates/application/tests/discover.rs`: genuine bound sockets,
//! no mocking library, no live internet access anywhere in this file.
//!
//! Covers the acceptance criteria Workstream 11 calls out explicitly:
//! partial files never appear as complete, interrupted downloads
//! recover via range-resume where the source supports it, a source
//! whose content changed invalidates a stale resume rather than
//! corrupting the file, checksum mismatches never finalize, and pinned
//! downloads survive quota-driven cleanup.

use std::net::SocketAddr;
use std::path::Path;

use application::{
    AppContext, DownloadService, OpenTarget, PlaybackService, PrivacyService, SettingsService,
    SourceService,
};
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use connectors::FEED_CONNECTOR_ID;
use domain::{ChecksumState, DownloadState, ItemId, MediaType, RemoteItem, SourceId, VariantId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn spawn_fixed_body_server(body: Vec<u8>) -> SocketAddr {
    let app = Router::new().route("/file.bin", get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// A raw TCP server (not axum) that declares `Content-Length: full_len`
/// but closes the connection after writing only `truncate_at` bytes —
/// a real premature-termination failure, not a simulated one.
async fn spawn_truncating_server(full_len: usize, truncate_at: usize) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 2048];
            let _ = socket.read(&mut buf).await;
            let body = fixture_bytes(full_len);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {full_len}\r\nConnection: close\r\n\r\n"
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&body[..truncate_at]).await;
            let _ = socket.flush().await;
            // Dropping the socket here closes the connection before
            // the promised Content-Length is satisfied.
        }
    });
    addr
}

fn fixture_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[derive(Clone)]
struct RangeServerState {
    full_body: &'static [u8],
    etag: &'static str,
}

async fn range_handler(State(state): State<RangeServerState>, headers: HeaderMap) -> Response {
    if let Some(range) = headers
        .get(axum::http::header::RANGE)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(start) = range
            .strip_prefix("bytes=")
            .and_then(|s| s.strip_suffix('-'))
            .and_then(|s| s.parse::<usize>().ok())
        {
            let start = start.min(state.full_body.len());
            let slice = &state.full_body[start..];
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header("ETag", state.etag)
                .header(
                    "Content-Range",
                    format!(
                        "bytes {}-{}/{}",
                        start,
                        state.full_body.len().saturating_sub(1),
                        state.full_body.len()
                    ),
                )
                .body(Body::from(slice.to_vec()))
                .unwrap();
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header("ETag", state.etag)
        .body(Body::from(state.full_body.to_vec()))
        .unwrap()
}

/// Honors `Range`/replies `206` — used by the resume test.
async fn spawn_range_server(full_body: &'static [u8], etag: &'static str) -> SocketAddr {
    let app = Router::new()
        .route("/file.bin", get(range_handler))
        .with_state(RangeServerState { full_body, etag });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn always_full_handler(State(state): State<RangeServerState>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("ETag", state.etag)
        .body(Body::from(state.full_body.to_vec()))
        .unwrap()
}

/// Always replies `200` with the full (current) body, ignoring any
/// `Range`/`If-Range` headers — simulates a source whose content
/// changed between attempts.
async fn spawn_always_full_server(full_body: &'static [u8], etag: &'static str) -> SocketAddr {
    let app = Router::new()
        .route("/file.bin", get(always_full_handler))
        .with_state(RangeServerState { full_body, etag });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

fn add_feed_source(ctx: &AppContext) -> SourceId {
    SourceService::add(
        ctx,
        FEED_CONNECTOR_ID,
        "Fixture Feed".to_string(),
        // Never actually fetched by these tests — `import_remote_item`
        // only needs the source to exist and resolve to a
        // downloads-capable connector, not to be browsed.
        serde_json::json!({ "url": "https://example.test/feed.xml" }),
    )
    .unwrap()
    .id
}

/// Imports a download-eligible `Video` item pointing at a real fixture
/// URL. `Video` (not `Story`, which is what `FeedConnector::browse`
/// always classifies feed entries as) so the completed download is
/// verifiable through `PlaybackService::resolve_open`'s external-player
/// path too.
fn import_downloadable_video(
    ctx: &AppContext,
    source_id: SourceId,
    source_item_id: &str,
    title: &str,
    download_url: &str,
    size: u64,
) -> (ItemId, VariantId) {
    let remote_item = RemoteItem {
        source_item_id: source_item_id.to_string(),
        title: title.to_string(),
        description: None,
        canonical_url: Some(format!("https://example.test/{source_item_id}")),
        tags: Vec::new(),
        media_type: MediaType::Video,
        thumbnail_url: None,
        download_url: Some(download_url.to_string()),
        download_mime_type: Some("video/mp4".to_string()),
        download_size_bytes: Some(size),
    };
    let item_id = SourceService::import_remote_item(ctx, source_id, remote_item).unwrap();
    let variant_id = {
        let conn = ctx.db.connection();
        let id: String = conn
            .query_row(
                "SELECT id FROM media_variants WHERE item_id = ?1",
                rusqlite::params![item_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        VariantId(uuid::Uuid::parse_str(&id).unwrap())
    };
    (item_id, variant_id)
}

#[tokio::test]
async fn end_to_end_download_completes_and_the_item_becomes_locally_openable() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let content = fixture_bytes(5000);
    let addr = spawn_fixed_body_server(content.clone()).await;
    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-e2e",
        "Fixture Video",
        &format!("http://{addr}/file.bin"),
        content.len() as u64,
    );

    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
    let finished = DownloadService::run(&ctx, download.id).await.unwrap();

    assert_eq!(finished.state, DownloadState::Completed);
    assert_eq!(finished.bytes_received, content.len() as u64);
    let bytes = std::fs::read(&finished.destination).unwrap();
    assert_eq!(bytes, content);

    match PlaybackService::resolve_open(&ctx, item_id).unwrap() {
        OpenTarget::ExternalPlayer { local_path, .. } => {
            assert_eq!(local_path, finished.destination);
        }
        other => panic!("expected ExternalPlayer target, got {other:?}"),
    }
}

#[tokio::test]
async fn partial_files_never_appear_complete_when_the_connection_is_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let full_len = 4000usize;
    let addr = spawn_truncating_server(full_len, 1000).await;
    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-truncated",
        "Truncated Video",
        &format!("http://{addr}/file.bin"),
        full_len as u64,
    );

    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
    let result = DownloadService::run(&ctx, download.id).await.unwrap();

    assert_ne!(result.state, DownloadState::Completed);
    assert!(
        !Path::new(&result.destination).exists(),
        "the final destination must never contain partial bytes"
    );
    let temp_path = result
        .temp_path
        .expect("an interrupted download retains its temp file");
    let partial = std::fs::read(&temp_path).unwrap();
    assert!(
        partial.len() < full_len,
        "the temp file should hold only the truncated bytes"
    );
}

#[tokio::test]
async fn range_resume_appends_only_the_missing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    const ETAG: &str = "\"fixture-etag\"";
    let full_body = fixture_bytes(3000);
    let full_body_static: &'static [u8] = Box::leak(full_body.clone().into_boxed_slice());
    let addr = spawn_range_server(full_body_static, ETAG).await;

    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-resume",
        "Resumable Video",
        &format!("http://{addr}/file.bin"),
        full_body.len() as u64,
    );
    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

    // Simulate a prior attempt that got partway through: a temp file
    // with the first 1000 bytes already on disk, and a `paused` row
    // recording the ETag from that attempt — the same state a real
    // pause mid-stream (or a crash) leaves behind.
    let temp_dir = ctx.data_dir.join("temp").join("downloads");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join(format!("{}.part", download.id));
    std::fs::write(&temp_path, &full_body[..1000]).unwrap();
    ctx.db
        .connection()
        .execute(
            "UPDATE downloads SET state = 'paused', temp_path = ?2, etag = ?3, bytes_received = 1000 \
             WHERE id = ?1",
            rusqlite::params![download.id.to_string(), temp_path.to_string_lossy(), ETAG],
        )
        .unwrap();

    let finished = DownloadService::resume(&ctx, download.id).await.unwrap();
    assert_eq!(finished.state, DownloadState::Completed);
    let bytes = std::fs::read(&finished.destination).unwrap();
    assert_eq!(
        bytes, full_body,
        "resumed content must exactly match the source, no duplication or corruption"
    );
}

#[tokio::test]
async fn resume_restarts_from_scratch_when_the_source_content_changed() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let new_body = fixture_bytes(2000)
        .into_iter()
        .map(|b| b.wrapping_add(1))
        .collect::<Vec<u8>>();
    let new_body_static: &'static [u8] = Box::leak(new_body.clone().into_boxed_slice());
    const NEW_ETAG: &str = "\"new-etag\"";
    let addr = spawn_always_full_server(new_body_static, NEW_ETAG).await;

    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-changed",
        "Changed Video",
        &format!("http://{addr}/file.bin"),
        new_body.len() as u64,
    );
    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

    // Seed a stale partial attempt: bytes that don't match the
    // server's current content, and a stale ETag the server no longer
    // recognizes.
    let temp_dir = ctx.data_dir.join("temp").join("downloads");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join(format!("{}.part", download.id));
    std::fs::write(&temp_path, vec![0xFFu8; 500]).unwrap();
    ctx.db
        .connection()
        .execute(
            "UPDATE downloads SET state = 'paused', temp_path = ?2, etag = 'stale-etag', bytes_received = 500 \
             WHERE id = ?1",
            rusqlite::params![download.id.to_string(), temp_path.to_string_lossy()],
        )
        .unwrap();

    let finished = DownloadService::resume(&ctx, download.id).await.unwrap();
    assert_eq!(finished.state, DownloadState::Completed);
    let bytes = std::fs::read(&finished.destination).unwrap();
    assert_eq!(
        bytes, new_body,
        "must restart from scratch rather than trusting the stale partial file"
    );
}

#[tokio::test]
async fn pinned_download_survives_quota_driven_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let content_a = vec![1u8; 1000];
    let content_b = vec![2u8; 1000];
    let addr_a = spawn_fixed_body_server(content_a.clone()).await;
    let addr_b = spawn_fixed_body_server(content_b.clone()).await;

    let source_id = add_feed_source(&ctx);
    let (item_a, variant_a) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-a",
        "Video A",
        &format!("http://{addr_a}/file.bin"),
        content_a.len() as u64,
    );
    let (item_b, variant_b) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-b",
        "Video B",
        &format!("http://{addr_b}/file.bin"),
        content_b.len() as u64,
    );

    let download_a = DownloadService::add(&ctx, item_a, variant_a).unwrap();
    let download_b = DownloadService::add(&ctx, item_b, variant_b).unwrap();
    let finished_a = DownloadService::run(&ctx, download_a.id).await.unwrap();
    let finished_b = DownloadService::run(&ctx, download_b.id).await.unwrap();
    assert_eq!(finished_a.state, DownloadState::Completed);
    assert_eq!(finished_b.state, DownloadState::Completed);

    DownloadService::set_pinned(&ctx, download_a.id, true).unwrap();
    SettingsService::set_download_quota_bytes(&ctx, Some(1000)).unwrap();

    let report = PrivacyService::enforce_download_quota(&ctx).unwrap();
    assert_eq!(report.evicted_files, 1);
    assert!(
        Path::new(&finished_a.destination).exists(),
        "pinned download must survive"
    );
    assert!(
        !Path::new(&finished_b.destination).exists(),
        "unpinned download must be evicted"
    );
}

#[tokio::test]
async fn checksum_mismatch_never_finalizes_the_download() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let content = vec![9u8; 2000];
    let addr = spawn_fixed_body_server(content.clone()).await;
    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-checksum",
        "Checksum Video",
        &format!("http://{addr}/file.bin"),
        content.len() as u64,
    );

    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
    ctx.db
        .connection()
        .execute(
            "UPDATE downloads SET expected_checksum = 'not-the-real-hash' WHERE id = ?1",
            rusqlite::params![download.id.to_string()],
        )
        .unwrap();

    let finished = DownloadService::run(&ctx, download.id).await.unwrap();
    assert_eq!(finished.state, DownloadState::Failed);
    assert_eq!(finished.checksum_state, ChecksumState::Mismatch);
    assert!(!Path::new(&finished.destination).exists());
    assert!(finished.temp_path.is_none());

    let temp_dir = ctx.data_dir.join("temp").join("downloads");
    let leftover = std::fs::read_dir(&temp_dir).map(|d| d.count()).unwrap_or(0);
    assert_eq!(
        leftover, 0,
        "no .part file should remain after a checksum mismatch"
    );
}

#[tokio::test]
async fn repair_stale_active_recovers_a_row_left_behind_by_a_killed_process() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open_at(dir.path()).unwrap();

    let full_body = fixture_bytes(4000);
    let addr = spawn_fixed_body_server(full_body.clone()).await;
    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-crash-recovery",
        "Crash Recovery Video",
        &format!("http://{addr}/file.bin"),
        full_body.len() as u64,
    );
    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

    // Simulate a process that claimed the row and then vanished
    // (SIGKILL, crash, power loss) partway through — no error handler
    // ever ran, so the row is left `active` with no fresh heartbeat,
    // exactly like `claim()`'s stuck-row bug produces in real use.
    let temp_dir = ctx.data_dir.join("temp").join("downloads");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join(format!("{}.part", download.id));
    std::fs::write(&temp_path, &full_body[..1500]).unwrap();
    let stale = application::time_format::to_rfc3339(
        time::OffsetDateTime::now_utc() - time::Duration::seconds(999),
    );
    ctx.db
        .connection()
        .execute(
            "UPDATE downloads SET state = 'active', temp_path = ?2, bytes_received = 1500, \
             updated_at = ?3 WHERE id = ?1",
            rusqlite::params![download.id.to_string(), temp_path.to_string_lossy(), stale],
        )
        .unwrap();

    // A fresh `run`/`resume` call before repair is a documented no-op —
    // this is the bug repair exists to fix.
    let untouched = DownloadService::run(&ctx, download.id).await.unwrap();
    assert_eq!(untouched.state, DownloadState::Active);

    let repaired =
        DownloadService::repair_stale_active(&ctx, std::time::Duration::from_secs(180)).unwrap();
    assert_eq!(repaired, vec![download.id]);

    let resumable = DownloadService::resumable_after_restart(&ctx).unwrap();
    assert_eq!(resumable, vec![download.id]);

    let finished = DownloadService::run(&ctx, download.id).await.unwrap();
    assert_eq!(finished.state, DownloadState::Completed);
    let bytes = std::fs::read(&finished.destination).unwrap();
    assert_eq!(bytes, full_body);
}
