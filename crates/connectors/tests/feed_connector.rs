//! End-to-end `FeedConnector` coverage: a real local HTTP server
//! (bound to an ephemeral 127.0.0.1 port, same pattern
//! `crates/local-api/tests/routes.rs` doesn't use directly but
//! mirrors — a genuine bound socket, not an in-process `oneshot`)
//! serves fixture RSS/Atom XML, and `FeedConnector` fetches + parses
//! it for real. No live internet access anywhere in this file.

use std::net::SocketAddr;

use axum::routing::get;
use axum::Router;
use connectors::{Connector, FeedConnector, FEED_CONNECTOR_ID};
use domain::{ConnectorResult, HealthState, Source, SourceId};

async fn spawn_fixture_server(body: &'static str) -> SocketAddr {
    spawn_owned_fixture_server(body.to_string()).await
}

async fn spawn_owned_fixture_server(body: String) -> SocketAddr {
    let app = Router::new().route("/feed.xml", get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

fn source_for(url: String) -> Source {
    Source {
        id: SourceId::new(),
        connector_id: FEED_CONNECTOR_ID,
        display_name: "Test Feed".to_string(),
        enabled: true,
        configuration_json: serde_json::json!({ "url": url }),
        credential_ref: None,
        health_state: HealthState::Unknown,
        last_health_check: None,
        capability_snapshot_json: None,
    }
}

#[tokio::test]
async fn browse_fetches_and_parses_a_real_rss_feed() {
    const RSS: &str = include_str!("../fixtures/sample_rss.xml");
    let addr = spawn_fixture_server(RSS).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].title, "First Story");
            assert_eq!(
                items[0].tags,
                vec!["fiction".to_string(), "short".to_string()]
            );
            assert_eq!(
                items[0].canonical_url.as_deref(),
                Some("https://example.test/first-story")
            );
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_fetches_and_parses_a_real_atom_feed() {
    const ATOM: &str = include_str!("../fixtures/sample_atom.xml");
    let addr = spawn_fixture_server(ATOM).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Atom Entry One");
            assert_eq!(
                items[0].canonical_url.as_deref(),
                Some("https://example.test/atom-one")
            );
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_extracts_enclosure_urls_from_a_real_feed() {
    const RSS: &str = include_str!("../fixtures/sample_rss_with_enclosure.xml");
    let addr = spawn_fixture_server(RSS).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].title, "Episode One");
            assert_eq!(
                items[0].download_url.as_deref(),
                Some("https://example.test/files/episode-one.mp3")
            );
            assert_eq!(items[0].download_mime_type.as_deref(), Some("audio/mpeg"));
            assert_eq!(items[0].download_size_bytes, Some(654321));

            assert_eq!(items[1].title, "Show Notes Only");
            assert_eq!(items[1].download_url, None);
        }
        other => panic!("expected Success, got {other:?}"),
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
    let source = source_for(format!("http://{addr}/missing.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_rejects_an_oversized_response() {
    // One byte over the connector's response-size cap (16 MiB) —
    // proves the limit is actually enforced against real bytes over a
    // real connection, not just documented.
    let oversized_body = "x".repeat(16 * 1024 * 1024 + 1);
    let addr = spawn_owned_fixture_server(oversized_body).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(msg) => {
            assert!(msg.contains("exceed"), "message should explain why: {msg}");
        }
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_maps_malformed_xml_to_a_permanent_failure() {
    let addr = spawn_fixture_server("not xml at all").await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(_) => {}
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn health_check_reports_healthy_for_a_reachable_feed() {
    const RSS: &str = include_str!("../fixtures/sample_rss.xml");
    let addr = spawn_fixture_server(RSS).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().health_check(&source).await {
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
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new().health_check(&source).await {
        ConnectorResult::Success(HealthState::Unreachable) => {}
        other => panic!("expected Success(Unreachable), got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_finds_a_specific_entry_by_id() {
    const RSS: &str = include_str!("../fixtures/sample_rss.xml");
    let addr = spawn_fixture_server(RSS).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new()
        .get_item(&source, "https://example.test/second-story")
        .await
    {
        ConnectorResult::Success(item) => assert_eq!(item.title, "Second Story"),
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_reports_not_found_for_an_unknown_id() {
    const RSS: &str = include_str!("../fixtures/sample_rss.xml");
    let addr = spawn_fixture_server(RSS).await;
    let source = source_for(format!("http://{addr}/feed.xml"));

    match FeedConnector::new()
        .get_item(&source, "does-not-exist")
        .await
    {
        ConnectorResult::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}
