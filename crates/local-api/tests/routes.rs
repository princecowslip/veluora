//! End-to-end coverage of the Milestone B routes, exercised through the
//! real router (not just unit-testing the underlying services).

use std::sync::Arc;

use application::AppContext;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use local_api::{build_router, generate_token, ApiState};
use serde_json::{json, Value};
use tower::ServiceExt as _;

async fn test_state() -> (ApiState, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
    let token = generate_token();
    let state = ApiState::new(ctx, Arc::from(token.as_str())).unwrap();
    (state, token, dir)
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
    let body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };
    let response = build_router(state.clone())
        .oneshot(builder.body(body).unwrap())
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
async fn library_root_lifecycle_via_http() {
    let (state, token, media_dir) = test_state().await;

    let (status, added) = call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let root_id = added["id"].as_str().unwrap().to_string();

    let (status, roots) = call(&state, &token, "GET", "/api/v1/library/roots", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(roots.as_array().unwrap().len(), 1);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/library/roots/{root_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, roots) = call(&state, &token, "GET", "/api/v1/library/roots", None).await;
    assert!(roots.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn scan_search_favorite_and_item_detail_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("clip.mp4"), b"video bytes").unwrap();

    let (status, _) = call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, report) = call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["roots"][0]["added"], 1);

    let (status, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:video" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(results["total"], 1);
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, item) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/items/{item_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(item["variants"].as_array().unwrap().len(), 1);
    assert_eq!(item["favorite"], false);

    let (status, updated) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/favorite"),
        Some(json!({ "favorite": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["favorite"], true);
}

#[tokio::test]
async fn collections_lifecycle_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("pic.png"), b"png bytes").unwrap();

    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:image" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, collection) = call(
        &state,
        &token,
        "POST",
        "/api/v1/collections",
        Some(json!({ "name": "Later" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let collection_id = collection["id"].as_str().unwrap().to_string();

    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/collections/{collection_id}/items"),
        Some(json!({ "item_id": item_id })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/collections/{collection_id}/items/{item_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/collections/{collection_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn unauthenticated_requests_to_new_routes_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/library/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn open_and_progress_round_trip_for_a_video_item() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("clip.mp4"), b"video bytes").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:video" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, target) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/open"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(target["kind"], "external_player");

    let (status, progress) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/progress"),
        Some(json!({ "progress_type": "time_based", "position_ms": 9_500, "duration_ms": 10_000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(progress["completed"], true);
}

#[tokio::test]
async fn story_route_returns_sanitized_content_and_chapter_map() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("tale.md"), "# Once\nUpon a time.\n").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:story" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, story) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/items/{item_id}/story"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(story["content"].as_str().unwrap().contains("Upon a time."));
    assert_eq!(story["chapter_map"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn pages_routes_list_and_serve_a_cbz_archive() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let (state, token, media_dir) = test_state().await;
    let cbz_path = media_dir.path().join("book.cbz");
    let file = std::fs::File::create(&cbz_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("000.jpg", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"page zero bytes").unwrap();
    writer.finish().unwrap();

    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:comic" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, pages) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/items/{item_id}/pages"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pages.as_array().unwrap().len(), 1);

    let response = build_router(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{item_id}/pages/0"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/jpeg"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"page zero bytes");
}

#[tokio::test]
async fn new_routes_404_on_an_unknown_item_and_400_on_a_bad_id() {
    let (state, token, _media_dir) = test_state().await;

    let unknown_id = uuid::Uuid::new_v4();
    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{unknown_id}/open"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = call(
        &state,
        &token,
        "GET",
        "/api/v1/items/not-a-uuid/pages",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unauthenticated_requests_to_the_new_routes_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let item_id = uuid::Uuid::new_v4();
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/items/{item_id}/open"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pin_toggle_round_trips_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("pic.png"), b"png bytes").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:image" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, updated) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/pin"),
        Some(json!({ "pinned": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["pinned"], true);

    let (status, updated) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/pin"),
        Some(json!({ "pinned": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["pinned"], false);
}

#[tokio::test]
async fn cache_status_quota_and_enforce_round_trip_via_http() {
    let (state, token, _media_dir) = test_state().await;

    let (status, status_body) = call(&state, &token, "GET", "/api/v1/cache/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_body["breakdown"]["total_bytes"], 0);
    assert!(status_body["quota_bytes"].is_null());

    let (status, _) = call(
        &state,
        &token,
        "POST",
        "/api/v1/cache/quota",
        Some(json!({ "bytes": 1024 })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, status_body) = call(&state, &token, "GET", "/api/v1/cache/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_body["quota_bytes"], 1024);

    let (status, report) = call(&state, &token, "POST", "/api/v1/cache/enforce-quota", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["evicted_files"], 0);
}

#[tokio::test]
async fn home_continue_lists_recently_opened_items_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("clip.mp4"), b"video bytes").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;
    let (_, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "type:video" })),
    )
    .await;
    let item_id = results["items"][0]["item_id"].as_str().unwrap().to_string();

    let (status, continue_before) =
        call(&state, &token, "GET", "/api/v1/home/continue", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(continue_before.as_array().unwrap().is_empty());

    call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/items/{item_id}/open"),
        None,
    )
    .await;

    let (status, continue_after) = call(&state, &token, "GET", "/api/v1/home/continue", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(continue_after.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn privacy_status_and_verify_round_trip_via_http() {
    let (state, token, _media_dir) = test_state().await;

    let (status, status_body) = call(&state, &token, "GET", "/api/v1/privacy/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_body["has_password"], false);
    assert_eq!(status_body["metadata_encryption_enabled"], false);

    let (status, verify_body) = call(
        &state,
        &token,
        "POST",
        "/api/v1/privacy/verify",
        Some(json!({ "password": "anything" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        verify_body["ok"], false,
        "verification fails closed when no password is set"
    );

    // Set a password directly through the application layer (there's no
    // HTTP route to set one — the TUI only ever verifies).
    application::PrivacyService::set_password(&state.ctx, "hunter2").unwrap();

    let (status, status_body) = call(&state, &token, "GET", "/api/v1/privacy/status", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_body["has_password"], true);

    let (status, verify_body) = call(
        &state,
        &token,
        "POST",
        "/api/v1/privacy/verify",
        Some(json!({ "password": "wrong" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(verify_body["ok"], false);

    let (status, verify_body) = call(
        &state,
        &token,
        "POST",
        "/api/v1/privacy/verify",
        Some(json!({ "password": "hunter2" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(verify_body["ok"], true);
}

const FEED_CONNECTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const LOCAL_FILESYSTEM_CONNECTOR_ID: &str = "00000000-0000-0000-0000-000000000000";

#[tokio::test]
async fn sources_lifecycle_via_http() {
    let (state, token, _media_dir) = test_state().await;

    let (status, source) = call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": FEED_CONNECTOR_ID,
            "display_name": "My Feed",
            "configuration_json": { "url": "https://example.test/feed.xml" },
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let source_id = source["id"].as_str().unwrap().to_string();

    let (status, sources) = call(&state, &token, "GET", "/api/v1/sources", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sources.as_array().unwrap().len(), 1);

    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/sources/{source_id}/disable"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/sources/{source_id}/enable"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/sources/{source_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, sources) = call(&state, &token, "GET", "/api/v1/sources", None).await;
    assert!(sources.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn local_source_health_check_and_browse_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("clip.mp4"), b"fake video bytes").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;

    let (status, source) = call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": LOCAL_FILESYSTEM_CONNECTOR_ID,
            "display_name": "Local Library",
            "configuration_json": {},
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let source_id = source["id"].as_str().unwrap().to_string();

    let (status, health) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/sources/{source_id}/health-check"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health, "healthy");

    let (status, report) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/sources/{source_id}/browse?query=type:video"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["result"]["status"], "success");
    assert_eq!(report["result"]["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn import_via_http_creates_a_searchable_item() {
    let (state, token, _media_dir) = test_state().await;

    let (_, source) = call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": FEED_CONNECTOR_ID,
            "display_name": "My Feed",
            "configuration_json": { "url": "https://example.test/feed.xml" },
        })),
    )
    .await;
    let source_id = source["id"].as_str().unwrap().to_string();

    let (status, imported) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/sources/{source_id}/import"),
        Some(json!({
            "source_item_id": "guid-1",
            "title": "Imported Story",
            "description": null,
            "canonical_url": "https://example.test/story",
            "tags": ["fiction"],
            "media_type": "story",
            "thumbnail_url": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let item_id = imported["item_id"].as_str().unwrap().to_string();

    let (status, results) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "Imported" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(results["total"], 1);
    assert_eq!(results["items"][0]["item_id"], item_id);
}

#[tokio::test]
async fn unauthenticated_requests_to_source_routes_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Regression guard for `docs/28-release-checklist.md`'s "cross-origin
/// protections pass" gate — `local-api` has no CORS layer at all, so a
/// browser page on another origin can't read responses even over
/// loopback. Confirms no permissive CORS layer has been accidentally
/// added: a request carrying a foreign `Origin` header must never get
/// an `Access-Control-Allow-Origin` header back.
#[tokio::test]
async fn no_route_ever_returns_a_permissive_cors_header() {
    let (state, token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header(header::ORIGIN, "https://evil.example")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "local-api must never grant a cross-origin page read access to its responses"
    );
}

#[tokio::test]
async fn malformed_search_query_is_a_bad_request() {
    let (state, token, _media_dir) = test_state().await;
    let (status, body) = call(
        &state,
        &token,
        "POST",
        "/api/v1/search",
        Some(json!({ "query": "bogus:value" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("bogus"));
}

#[tokio::test]
async fn discover_combines_local_and_connector_hits_via_http() {
    let (state, token, media_dir) = test_state().await;
    std::fs::write(media_dir.path().join("clip.mp4"), b"fake video bytes").unwrap();
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/roots",
        Some(json!({ "path": media_dir.path().to_string_lossy() })),
    )
    .await;
    call(
        &state,
        &token,
        "POST",
        "/api/v1/library/scan",
        Some(json!({})),
    )
    .await;

    call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": FEED_CONNECTOR_ID,
            "display_name": "My Feed",
            "configuration_json": { "url": "http://127.0.0.1:1/does-not-exist.xml" },
        })),
    )
    .await;

    let (status, report) = call(
        &state,
        &token,
        "POST",
        "/api/v1/discover",
        Some(json!({ "query": "type:video" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The local scanned item is a hit; the unreachable feed source
    // contributes none but is still reported, not silently dropped.
    let hits = report["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0]["local_item_id"].is_string());

    let sources = report["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .any(|s| s["source_display_name"] == "My Feed" && s["status"]["status"] != "success"));
}

#[tokio::test]
async fn discover_with_a_malformed_query_is_a_bad_request() {
    let (state, token, _media_dir) = test_state().await;
    let (status, body) = call(
        &state,
        &token,
        "POST",
        "/api/v1/discover",
        Some(json!({ "query": "bogus:value" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("bogus"));
}

#[tokio::test]
async fn discover_with_an_invalid_source_id_is_a_bad_request() {
    let (state, token, _media_dir) = test_state().await;
    let (status, _) = call(
        &state,
        &token,
        "POST",
        "/api/v1/discover",
        Some(json!({ "query": "hello", "source_ids": ["not-a-uuid"] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unauthenticated_requests_to_discover_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/discover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "query": "" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

async fn spawn_fixture_download_server(body: Vec<u8>) -> std::net::SocketAddr {
    let app =
        axum::Router::new().route("/file.bin", axum::routing::get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn download_lifecycle_via_http() {
    let (state, token, _media_dir) = test_state().await;
    let content = vec![7u8; 4096];
    let addr = spawn_fixture_download_server(content.clone()).await;

    let (_, source) = call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": FEED_CONNECTOR_ID,
            "display_name": "My Feed",
            "configuration_json": { "url": "https://example.test/feed.xml" },
        })),
    )
    .await;
    let source_id = source["id"].as_str().unwrap().to_string();

    let (status, imported) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/sources/{source_id}/import"),
        Some(json!({
            "source_item_id": "guid-1",
            "title": "Downloadable Episode",
            "description": null,
            "canonical_url": "https://example.test/episode",
            "tags": [],
            "media_type": "video",
            "thumbnail_url": null,
            "download_url": format!("http://{addr}/file.bin"),
            "download_mime_type": "video/mp4",
            "download_size_bytes": content.len(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let item_id = imported["item_id"].as_str().unwrap().to_string();

    let (_, item) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/items/{item_id}"),
        None,
    )
    .await;
    let variant_id = item["variants"][0]["id"].as_str().unwrap().to_string();

    let (status, eligibility) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/downloads/eligibility?item_id={item_id}&variant_id={variant_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(eligibility["eligible"], true);

    let (status, download) = call(
        &state,
        &token,
        "POST",
        "/api/v1/downloads",
        Some(json!({ "item_id": item_id, "variant_id": variant_id })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let download_id = download["id"].as_str().unwrap().to_string();

    let mut final_download = json!(null);
    for _ in 0..50 {
        let (_, found) = call(
            &state,
            &token,
            "GET",
            &format!("/api/v1/downloads/{download_id}"),
            None,
        )
        .await;
        if found["state"] == "completed" || found["state"] == "failed" {
            final_download = found;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(final_download["state"], "completed", "{final_download}");

    let (status, list) = call(&state, &token, "GET", "/api/v1/downloads", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/downloads/{download_id}?delete_file=true"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "GET",
        &format!("/api/v1/items/{item_id}"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the library item must survive deleting its download"
    );
}

#[tokio::test]
async fn downloads_eligibility_reports_ineligible_for_an_unknown_variant() {
    let (state, token, _media_dir) = test_state().await;
    let (_, source) = call(
        &state,
        &token,
        "POST",
        "/api/v1/sources",
        Some(json!({
            "connector_id": FEED_CONNECTOR_ID,
            "display_name": "My Feed",
            "configuration_json": { "url": "https://example.test/feed.xml" },
        })),
    )
    .await;
    let _ = source;

    let unknown_item = uuid::Uuid::new_v4();
    let unknown_variant = uuid::Uuid::new_v4();
    let (status, eligibility) = call(
        &state,
        &token,
        "GET",
        &format!(
            "/api/v1/downloads/eligibility?item_id={unknown_item}&variant_id={unknown_variant}"
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(eligibility["eligible"], false);
}

#[tokio::test]
async fn unauthenticated_requests_to_download_routes_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/downloads")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn block_rules_lifecycle_via_http() {
    let (state, token, _media_dir) = test_state().await;

    let (status, rule) = call(
        &state,
        &token,
        "POST",
        "/api/v1/block-rules",
        Some(json!({
            "rule_type": "tag",
            "target": "blocked-tag",
            "scope": "all",
            "reason": "test rule",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(rule["enabled"], true);
    let rule_id = rule["id"].as_str().unwrap().to_string();

    let (status, rules) = call(&state, &token, "GET", "/api/v1/block-rules", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rules.as_array().unwrap().len(), 1);

    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/block-rules/{rule_id}/disable"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "POST",
        &format!("/api/v1/block-rules/{rule_id}/enable"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(
        &state,
        &token,
        "DELETE",
        &format!("/api/v1/block-rules/{rule_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, rules) = call(&state, &token, "GET", "/api/v1/block-rules", None).await;
    assert!(rules.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn unauthenticated_requests_to_block_rule_routes_are_rejected() {
    let (state, _token, _media_dir) = test_state().await;
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/block-rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
