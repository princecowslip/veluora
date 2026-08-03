//! Coverage for the startup download-recovery path and the
//! `max_concurrent_downloads` cap added to fix the gap `KNOWN_ISSUES.md`
//! flags: no auto-resume across a restart, and no scheduler to enforce
//! a concurrency limit. See `local_api::recover_and_resume_downloads`
//! and `routes::downloads::spawn_download`.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use application::{AppContext, DownloadService, SettingsService, SourceService};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use connectors::FEED_CONNECTOR_ID;
use domain::{ItemId, RemoteItem, VariantId};
use local_api::{build_router, generate_token, recover_and_resume_downloads, ApiState};
use serde_json::{json, Value};
use tower::ServiceExt as _;

fn add_feed_source(ctx: &AppContext) -> domain::SourceId {
    SourceService::add(
        ctx,
        FEED_CONNECTOR_ID,
        "Recovery Fixture Feed".to_string(),
        serde_json::json!({ "url": "https://example.test/feed.xml" }),
    )
    .unwrap()
    .id
}

fn import_downloadable_video(
    ctx: &AppContext,
    source_id: domain::SourceId,
    source_item_id: &str,
    download_url: &str,
    size: u64,
) -> (ItemId, VariantId) {
    let remote_item = RemoteItem {
        source_item_id: source_item_id.to_string(),
        title: format!("Video {source_item_id}"),
        description: None,
        canonical_url: Some(format!("https://example.test/{source_item_id}")),
        tags: Vec::new(),
        media_type: domain::MediaType::Video,
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

async fn spawn_fixed_body_server(body: Vec<u8>) -> SocketAddr {
    let app =
        axum::Router::new().route("/file.bin", axum::routing::get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// Serves the body in several chunks with a delay between each, so a
/// concurrency-cap test has a real window in which to observe more than
/// one transfer `active` at once.
async fn spawn_slow_server(body: Vec<u8>, chunk_delay: Duration) -> SocketAddr {
    use futures::{stream, StreamExt};
    let chunks: Vec<Vec<u8>> = body.chunks(512).map(|c| c.to_vec()).collect();
    let app = axum::Router::new().route(
        "/file.bin",
        axum::routing::get(move || {
            let chunks = chunks.clone();
            async move {
                let stream = stream::iter(chunks).then(move |chunk| async move {
                    tokio::time::sleep(chunk_delay).await;
                    Ok::<_, std::io::Error>(chunk)
                });
                Body::from_stream(stream)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn call(
    state: &ApiState,
    token: &str,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let request_body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };
    let response = build_router(state.clone())
        .oneshot(builder.body(request_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json_body)
}

#[tokio::test]
async fn recover_and_resume_downloads_completes_a_row_left_stuck_by_a_killed_process() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());

    let full_body = vec![5u8; 4000];
    let addr = spawn_fixed_body_server(full_body.clone()).await;
    let source_id = add_feed_source(&ctx);
    let (item_id, variant_id) = import_downloadable_video(
        &ctx,
        source_id,
        "guid-recovery",
        &format!("http://{addr}/file.bin"),
        full_body.len() as u64,
    );
    let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

    // Simulate `local-api` being killed mid-transfer: the row is left
    // `active` with a stale heartbeat and no process left to drive it —
    // exactly the stuck-row bug `claim()` alone can't recover from.
    let temp_dir = ctx.data_dir.join("temp").join("downloads");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join(format!("{}.part", download.id));
    std::fs::write(&temp_path, &full_body[..2000]).unwrap();
    let stale = application::time_format::to_rfc3339(
        time::OffsetDateTime::now_utc() - time::Duration::seconds(999),
    );
    ctx.db
        .connection()
        .execute(
            "UPDATE downloads SET state = 'active', temp_path = ?2, bytes_received = 2000, \
             updated_at = ?3 WHERE id = ?1",
            rusqlite::params![download.id.to_string(), temp_path.to_string_lossy(), stale],
        )
        .unwrap();

    // A fresh process (or a fresh `ApiState` in the same one) starting
    // up now runs the same recovery path `main.rs` runs at startup.
    let state = ApiState::new(ctx.clone(), Arc::from(generate_token())).unwrap();
    let recovered = recover_and_resume_downloads(&state.ctx, &state.download_semaphore).unwrap();
    assert_eq!(recovered, vec![download.id]);

    let mut final_state = String::new();
    for _ in 0..100 {
        let found = DownloadService::find(&ctx, download.id).unwrap().unwrap();
        final_state = format!("{:?}", found.state);
        if found.state == domain::DownloadState::Completed {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(final_state, "Completed");

    let finished = DownloadService::find(&ctx, download.id).unwrap().unwrap();
    let bytes = std::fs::read(&finished.destination).unwrap();
    assert_eq!(bytes, full_body);
}

#[tokio::test]
async fn max_concurrent_downloads_caps_how_many_run_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
    SettingsService::set_max_concurrent_downloads(&ctx, 2).unwrap();

    let token = generate_token();
    let state = ApiState::new(ctx.clone(), Arc::from(token.as_str())).unwrap();

    let body = vec![9u8; 2048];
    let mut download_ids = Vec::new();
    for i in 0..5 {
        let addr = spawn_slow_server(body.clone(), Duration::from_millis(15)).await;
        let source_id = add_feed_source(&ctx);
        let (item_id, variant_id) = import_downloadable_video(
            &ctx,
            source_id,
            &format!("guid-cap-{i}"),
            &format!("http://{addr}/file.bin"),
            body.len() as u64,
        );
        let (status, download) = call(
            &state,
            &token,
            "POST",
            "/api/v1/downloads",
            Some(json!({ "item_id": item_id.to_string(), "variant_id": variant_id.to_string() })),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        download_ids.push(download["id"].as_str().unwrap().to_string());
    }

    let mut max_observed_active = 0usize;
    let mut all_completed = false;
    for _ in 0..200 {
        let (_, list) = call(&state, &token, "GET", "/api/v1/downloads", None).await;
        let rows = list.as_array().unwrap();
        let active = rows.iter().filter(|r| r["state"] == "active").count();
        max_observed_active = max_observed_active.max(active);
        if rows.iter().all(|r| r["state"] == "completed") {
            all_completed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(all_completed, "all 5 downloads must eventually complete");
    assert!(
        max_observed_active <= 2,
        "at most max_concurrent_downloads (2) should ever be active at once, saw {max_observed_active}"
    );
}
