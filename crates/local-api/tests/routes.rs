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
