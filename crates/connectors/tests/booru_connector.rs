//! End-to-end `BooruConnector` coverage: a real local HTTP server
//! (bound to an ephemeral 127.0.0.1 port) serves fixture Danbooru/
//! Gelbooru-shaped JSON, and `BooruConnector` fetches + parses it for
//! real — same pattern as `feed_connector.rs`. No live internet access
//! anywhere in this file.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::Query;
use axum::routing::get;
use axum::Router;
use connectors::{BooruConnector, Connector, BOORU_CONNECTOR_ID};
use domain::{ConnectorResult, HealthState, Source, SourceId};
use tokio::sync::Mutex;

fn source_for(base_url: String, flavor: &str) -> Source {
    Source {
        id: SourceId::new(),
        connector_id: BOORU_CONNECTOR_ID,
        display_name: "Test Booru".to_string(),
        enabled: true,
        configuration_json: serde_json::json!({ "flavor": flavor, "base_url": base_url }),
        credential_ref: None,
        health_state: HealthState::Unknown,
        last_health_check: None,
        capability_snapshot_json: None,
    }
}

async fn spawn_router(app: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn spawn_fixed_body_server(path: &'static str, body: &'static str) -> SocketAddr {
    let app = Router::new().route(path, get(move || async move { body }));
    spawn_router(app).await
}

/// Serves `body` at `path` and captures the decoded `tags` query
/// parameter of the request that fetched it, so search-translation
/// tests can assert on the exact string the connector sent.
async fn spawn_tags_capturing_server(
    path: &'static str,
    body: String,
) -> (SocketAddr, Arc<Mutex<Option<String>>>) {
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_for_handler = captured.clone();
    let app = Router::new().route(
        path,
        get(move |Query(params): Query<HashMap<String, String>>| {
            let captured = captured_for_handler.clone();
            let body = body.clone();
            async move {
                *captured.lock().await = params.get("tags").cloned();
                body
            }
        }),
    );
    let addr = spawn_router(app).await;
    (addr, captured)
}

#[tokio::test]
async fn browse_fetches_and_parses_danbooru_posts() {
    const BODY: &str = include_str!("../fixtures/danbooru_posts.json");
    let addr = spawn_fixed_body_server("/posts.json", BODY).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].source_item_id, "1001");
            assert_eq!(items[0].title, "Post #1001");
            assert!(items[0].tags.contains(&"rating:general".to_string()));
            assert_eq!(
                items[0].download_url.as_deref(),
                Some("https://cdn.example.test/1001.jpg")
            );
            assert_eq!(items[0].download_mime_type.as_deref(), Some("image/jpeg"));
            assert_eq!(items[0].download_size_bytes, Some(234567));
            assert_eq!(items[1].media_type, domain::MediaType::Video);
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_fetches_and_parses_gelbooru_posts_as_a_bare_array() {
    const BODY: &str = include_str!("../fixtures/gelbooru_posts_array.json");
    let addr = spawn_fixed_body_server("/index.php", BODY).await;
    let source = source_for(format!("http://{addr}"), "gelbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].source_item_id, "2001");
            assert!(items[0].tags.contains(&"rating:general".to_string()));
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_fetches_and_parses_gelbooru_posts_wrapped_in_a_post_object() {
    const BODY: &str = include_str!("../fixtures/gelbooru_posts_wrapped.json");
    let addr = spawn_fixed_body_server("/index.php", BODY).await;
    let source = source_for(format!("http://{addr}"), "gelbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].source_item_id, "2002");
            assert!(items[0].tags.contains(&"rating:questionable".to_string()));
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_handles_a_null_post_key_as_an_empty_result_set() {
    let addr = spawn_fixed_body_server("/index.php", r#"{"post": null}"#).await;
    let source = source_for(format!("http://{addr}"), "gelbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::Success(items) => assert!(items.is_empty()),
        other => panic!("expected Success(empty), got {other:?}"),
    }
}

#[tokio::test]
async fn search_translates_free_text_and_tag_filters_into_a_tags_query_param() {
    const BODY: &str = include_str!("../fixtures/danbooru_posts.json");
    let (addr, captured) = spawn_tags_capturing_server("/posts.json", BODY.to_string()).await;
    let source = source_for(format!("http://{addr}"), "danbooru");
    let query = domain::parse_search_query("blue_eyes -tag:blocked").unwrap();

    match BooruConnector::new().search(&source, &query).await {
        ConnectorResult::Success(_) => {}
        other => panic!("expected Success, got {other:?}"),
    }
    assert_eq!(captured.lock().await.as_deref(), Some("blue_eyes -blocked"));
}

#[tokio::test]
async fn search_reports_unsupported_query_for_a_field_filter_the_connector_cannot_translate() {
    // No server needed — the connector rejects the query before making
    // any HTTP call, so an unroutable address proves nothing was sent.
    let source = source_for("http://127.0.0.1:1".to_string(), "danbooru");
    let query = domain::parse_search_query("width:>1920").unwrap();

    match BooruConnector::new().search(&source, &query).await {
        ConnectorResult::UnsupportedQuery => {}
        other => panic!("expected UnsupportedQuery, got {other:?}"),
    }
}

#[tokio::test]
async fn search_reports_unsupported_query_for_an_or_group() {
    let source = source_for("http://127.0.0.1:1".to_string(), "danbooru");
    let query = domain::parse_search_query("(cat OR dog)").unwrap();

    match BooruConnector::new().search(&source, &query).await {
        ConnectorResult::UnsupportedQuery => {}
        other => panic!("expected UnsupportedQuery, got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_fetches_a_single_danbooru_post_by_id() {
    const BODY: &str = include_str!("../fixtures/danbooru_post_single.json");
    let addr = spawn_fixed_body_server("/posts/1001.json", BODY).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().get_item(&source, "1001").await {
        ConnectorResult::Success(item) => assert_eq!(item.source_item_id, "1001"),
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_item_translates_to_an_id_tag_query_for_gelbooru() {
    const BODY: &str = include_str!("../fixtures/gelbooru_posts_array.json");
    let (addr, captured) = spawn_tags_capturing_server("/index.php", BODY.to_string()).await;
    let source = source_for(format!("http://{addr}"), "gelbooru");

    match BooruConnector::new().get_item(&source, "2001").await {
        ConnectorResult::Success(item) => assert_eq!(item.source_item_id, "2001"),
        other => panic!("expected Success, got {other:?}"),
    }
    assert_eq!(captured.lock().await.as_deref(), Some("id:2001"));
}

#[tokio::test]
async fn get_tags_returns_matching_tag_names_for_danbooru() {
    const BODY: &str = include_str!("../fixtures/danbooru_tags.json");
    let addr = spawn_fixed_body_server("/tags.json", BODY).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().get_tags(&source, "blue").await {
        ConnectorResult::Success(tags) => {
            assert_eq!(tags, vec!["blue_eyes".to_string(), "blue_hair".to_string()]);
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn get_tags_returns_matching_tag_names_for_gelbooru() {
    const BODY: &str = include_str!("../fixtures/gelbooru_tags.json");
    let addr = spawn_fixed_body_server("/index.php", BODY).await;
    let source = source_for(format!("http://{addr}"), "gelbooru");

    match BooruConnector::new().get_tags(&source, "dog").await {
        ConnectorResult::Success(tags) => {
            assert_eq!(tags, vec!["dog".to_string(), "dog_park".to_string()]);
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_without_a_configured_base_url_is_a_permanent_failure() {
    let mut source = source_for("http://127.0.0.1:1".to_string(), "danbooru");
    source.configuration_json = serde_json::json!({ "flavor": "danbooru" });

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(_) => {}
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_maps_a_404_to_not_found() {
    let app = Router::new(); // no routes registered — every request 404s
    let addr = spawn_router(app).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().browse(&source, None).await {
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
    let app = Router::new().route("/posts.json", get(move || async move { oversized_body }));
    let addr = spawn_router(app).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(msg) => {
            assert!(msg.contains("exceed"), "message should explain why: {msg}");
        }
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn browse_maps_malformed_json_to_a_permanent_failure() {
    let addr = spawn_fixed_body_server("/posts.json", "not json at all").await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().browse(&source, None).await {
        ConnectorResult::PermanentFailure(_) => {}
        other => panic!("expected PermanentFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn health_check_reports_healthy_for_a_reachable_instance() {
    const BODY: &str = include_str!("../fixtures/danbooru_posts.json");
    let addr = spawn_fixed_body_server("/posts.json", BODY).await;
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().health_check(&source).await {
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
    let source = source_for(format!("http://{addr}"), "danbooru");

    match BooruConnector::new().health_check(&source).await {
        ConnectorResult::Success(HealthState::Unreachable) => {}
        other => panic!("expected Success(Unreachable), got {other:?}"),
    }
}
