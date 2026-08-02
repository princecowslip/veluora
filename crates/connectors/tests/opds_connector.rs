//! End-to-end `OpdsConnector` coverage: a real local HTTP server (bound
//! to an ephemeral 127.0.0.1 port) serves fixture OPDS catalog XML, and
//! `OpdsConnector` fetches + parses it for real, following the same
//! pattern `tests/feed_connector.rs`/`tests/booru_connector.rs` use. No
//! live internet access anywhere in this file.

use std::net::SocketAddr;

use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::Router;
use connectors::{Connector, OpdsConnector, OPDS_CONNECTOR_ID};
use domain::{ConnectorResult, HealthState, MediaType, Source, SourceId};

async fn spawn_fixture_server(body: &'static str) -> SocketAddr {
    spawn_owned_fixture_server(body.to_string()).await
}

async fn spawn_owned_fixture_server(body: String) -> SocketAddr {
    let app = Router::new().route("/catalog", get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// A full three-route catalog: a root navigation feed with a `next`
/// page, a second navigation page, and an acquisition sub-feed — enough
/// to exercise `browse`'s cursor pagination and `get_gallery`'s
/// drill-down together.
async fn spawn_catalog_server() -> SocketAddr {
    const ROOT: &str = include_str!("../fixtures/opds_root.xml");
    const PAGE2: &str = include_str!("../fixtures/opds_root_page2.xml");
    const SERIES: &str = include_str!("../fixtures/opds_series.xml");
    let app = Router::new()
        .route("/catalog", get(|| async { ROOT }))
        .route("/catalog/page2", get(|| async { PAGE2 }))
        .route("/series/1", get(|| async { SERIES }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// A single route that requires HTTP Basic auth (`reader`/`hunter2`),
/// serving the acquisition fixture only when the header matches.
async fn spawn_protected_server() -> SocketAddr {
    async fn handler(headers: HeaderMap) -> (StatusCode, &'static str) {
        const SERIES: &str = include_str!("../fixtures/opds_series.xml");
        let expected = format!("Basic {}", base64_encode(b"reader:hunter2"));
        match headers.get(axum::http::header::AUTHORIZATION) {
            Some(value) if value.to_str().ok() == Some(expected.as_str()) => {
                (StatusCode::OK, SERIES)
            }
            _ => (StatusCode::UNAUTHORIZED, ""),
        }
    }
    let app = Router::new().route("/catalog", get(handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// A tiny dependency-free base64 encoder — avoids adding a dev-only
/// `base64` crate just to build the one expected `Authorization` header
/// value this test file compares against.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4)) as usize] as char);
        out.push(match b1 {
            Some(b1) => ALPHABET[(((b1 & 0x0f) << 2) | (b2.unwrap_or(0) >> 6)) as usize] as char,
            None => '=',
        });
        out.push(match b2 {
            Some(b2) => ALPHABET[(b2 & 0x3f) as usize] as char,
            None => '=',
        });
    }
    out
}

fn source_for(config: serde_json::Value) -> Source {
    Source {
        id: SourceId::new(),
        connector_id: OPDS_CONNECTOR_ID,
        display_name: "Test OPDS".to_string(),
        enabled: true,
        configuration_json: config,
        credential_ref: None,
        health_state: HealthState::Unknown,
        last_health_check: None,
        capability_snapshot_json: None,
    }
}

fn url_source(url: String) -> Source {
    source_for(serde_json::json!({ "url": url }))
}

#[tokio::test]
async fn browse_fetches_the_root_catalog_and_resolves_relative_links() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Fantasy Series");
            assert_eq!(items[0].media_type, MediaType::Gallery);
            // A navigation entry's `source_item_id` is the resolved,
            // absolute sub-feed URL — relative in the fixture XML,
            // resolved against the catalog's own URL via `base_uri`.
            assert_eq!(items[0].source_item_id, format!("http://{addr}/series/1"));
            assert_eq!(
                items[0].thumbnail_url.as_deref(),
                Some(format!("http://{addr}/series/1/cover.jpg").as_str())
            );
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_follows_a_cursor_to_the_next_page() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));
    let connector = OpdsConnector::new();

    let cursor = format!("http://{addr}/catalog/page2");
    match connector.browse(&source, Some(&cursor)).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Sci-Fi Series");
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_gallery_drills_into_a_navigation_entrys_sub_feed() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));
    let connector = OpdsConnector::new();

    let root_items = match connector.browse(&source, None).await {
        ConnectorResult::Success(items) => items,
        other => panic!("expected Success, got {other:?}"),
    };
    let gallery_id = &root_items[0].source_item_id;

    match connector.get_gallery(&source, gallery_id).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 2);

            assert_eq!(items[0].title, "Volume One");
            assert_eq!(items[0].media_type, MediaType::Story);
            assert_eq!(
                items[0].download_url.as_deref(),
                Some(format!("http://{addr}/series/1/volume-1.epub").as_str())
            );
            assert_eq!(
                items[0].download_mime_type.as_deref(),
                Some("application/epub+zip")
            );
            assert_eq!(items[0].download_size_bytes, Some(204800));
            assert_eq!(
                items[0].tags,
                vec!["fantasy".to_string(), "adventure".to_string()]
            );

            assert_eq!(items[1].title, "Volume Two");
            assert_eq!(items[1].media_type, MediaType::Comic);
            assert_eq!(
                items[1].download_url.as_deref(),
                Some(format!("http://{addr}/series/1/volume-2.cbz").as_str())
            );
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_finds_an_entry_in_the_root_catalog() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new()
        .get_item(&source, &format!("http://{addr}/series/1"))
        .await
    {
        ConnectorResult::Success(item) => assert_eq!(item.title, "Fantasy Series"),
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_reports_not_found_for_an_unknown_id() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new()
        .get_item(&source, "does-not-exist")
        .await
    {
        ConnectorResult::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_sends_basic_auth_credentials_when_configured() {
    let addr = spawn_protected_server().await;
    let source = source_for(serde_json::json!({
        "url": format!("http://{addr}/catalog"),
        "username": "reader",
        "password": "hunter2",
    }));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => assert_eq!(items.len(), 2),
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_without_credentials_against_a_protected_catalog_requires_authentication() {
    let addr = spawn_protected_server().await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::AuthenticationRequired => {}
        other => panic!("expected AuthenticationRequired, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_without_a_configured_url_is_a_permanent_failure() {
    let connector = OpdsConnector::new();
    let source = source_for(serde_json::json!({}));
    match connector.browse(&source, None).await {
        ConnectorResult::PermanentFailure(_) => {}
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_maps_a_404_to_not_found() {
    let app = Router::new(); // no routes registered — every request 404s
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_rejects_an_oversized_response() {
    // One byte over the connector's response-size cap (16 MiB) — proves
    // the limit is actually enforced against real bytes over a real
    // connection, not just documented.
    let oversized_body = "x".repeat(16 * 1024 * 1024 + 1);
    let addr = spawn_owned_fixture_server(oversized_body).await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(msg) => {
            assert!(msg.contains("exceed"), "message should explain why: {msg}");
        }
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_maps_malformed_xml_to_a_permanent_failure() {
    let addr = spawn_fixture_server("not xml at all").await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(_) => {}
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn health_check_reports_healthy_for_a_reachable_catalog() {
    let addr = spawn_catalog_server().await;
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().health_check(&source).await {
        ConnectorResult::Success(HealthState::Healthy) => {}
        other => panic!("expected Success(Healthy), got {other:?}"),
    }
}

#[tokio::test]
async fn health_check_reports_unreachable_when_nothing_is_listening() {
    // Bind then immediately drop the listener so the port is (very
    // likely) refused — a real "nothing there" case, not a mock.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let source = url_source(format!("http://{addr}/catalog"));

    match OpdsConnector::new().health_check(&source).await {
        ConnectorResult::Success(HealthState::Unreachable) => {}
        other => panic!("expected Success(Unreachable), got {other:?}"),
    }
}
