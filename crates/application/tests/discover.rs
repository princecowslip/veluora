//! End-to-end `DiscoverService` coverage against two real local HTTP
//! fixture servers — one serving valid RSS, one unreachable — proving
//! that a broken connector doesn't suppress hits from a working one or
//! abort the whole aggregate (`docs/46-implementation-plan.md`
//! Workstream 10's "connector failures are isolated" acceptance
//! criterion). Same approach as
//! `crates/connectors/tests/feed_connector.rs`: real bound sockets, no
//! live internet access anywhere in this file.

use std::net::SocketAddr;

use application::{AppContext, DiscoverService, SourceService};
use axum::routing::get;
use axum::Router;
use connectors::FEED_CONNECTOR_ID;
use domain::ConnectorResult;

async fn spawn_fixture_server(body: &'static str) -> SocketAddr {
    let app = Router::new().route("/feed.xml", get(move || async move { body }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

fn insert_media_item(ctx: &AppContext, title: &str, media_type: &str) {
    ctx.db
        .connection()
        .execute(
            "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
             VALUES (?1, ?2, ?3, 'unrated', datetime('now'), datetime('now'))",
            rusqlite::params![domain::ItemId::new().to_string(), media_type, title],
        )
        .unwrap();
}

#[tokio::test]
async fn a_broken_connector_does_not_suppress_hits_from_a_working_one() {
    const RSS: &str = include_str!("fixtures/sample_rss.xml");
    let working_addr = spawn_fixture_server(RSS).await;

    // Bind then immediately drop so the port is (very likely) refused
    // — a real "nothing there" failure, not a mock.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let broken_addr = listener.local_addr().unwrap();
    drop(listener);

    let ctx = AppContext::open_in_memory().unwrap();
    insert_media_item(&ctx, "A Local Video", "video");

    let working_source = SourceService::add(
        &ctx,
        FEED_CONNECTOR_ID,
        "Working Feed".to_string(),
        serde_json::json!({ "url": format!("http://{working_addr}/feed.xml") }),
    )
    .unwrap();
    let broken_source = SourceService::add(
        &ctx,
        FEED_CONNECTOR_ID,
        "Broken Feed".to_string(),
        serde_json::json!({ "url": format!("http://{broken_addr}/feed.xml") }),
    )
    .unwrap();

    let report = DiscoverService::discover(&ctx, "", None, 25).await.unwrap();

    // The local item and the working feed's two entries all appear...
    assert_eq!(report.hits.len(), 3);
    assert!(report.hits.iter().any(|h| h.item.title == "A Local Video"));
    assert!(report.hits.iter().any(|h| h.item.title == "First Story"));
    assert!(report.hits.iter().any(|h| h.item.title == "Second Story"));

    // ...and the broken source is reported as failed, not silently
    // dropped and not aborting the whole aggregate.
    let working_status = report
        .sources
        .iter()
        .find(|s| s.source_id == working_source.id)
        .unwrap();
    assert!(matches!(working_status.status, ConnectorResult::Success(2)));

    let broken_status = report
        .sources
        .iter()
        .find(|s| s.source_id == broken_source.id)
        .unwrap();
    assert!(!matches!(broken_status.status, ConnectorResult::Success(_)));
}
