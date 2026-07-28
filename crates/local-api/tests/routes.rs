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
    let state = ApiState {
        ctx,
        token: Arc::from(token.as_str()),
    };
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
