//! An RSS/Atom feed connector — one of `docs/14`'s recommended
//! reference connectors, and fully testable offline (see
//! `tests/feed_connector.rs`, which serves fixture feed XML from a
//! real local HTTP server rather than depending on live internet
//! access).

use async_trait::async_trait;
use domain::{
    AuthMethod, ConnectorCapabilities, ConnectorId, ConnectorResult, HealthState, MediaType,
    PaginationMode, RemoteItem, Source,
};
use uuid::Uuid;

use crate::Connector;

/// Fixed so a `Source.connector_id` reliably identifies "this is the
/// feed connector" across restarts — connectors aren't looked up by
/// name at runtime, only by this stable id.
pub const FEED_CONNECTOR_ID: ConnectorId = ConnectorId(Uuid::from_u128(1));

/// A malicious or misconfigured feed shouldn't be able to exhaust
/// memory — `docs/22-testing-strategy.md`'s "oversized response"
/// security test, and `docs/14-source-connectors.md`'s "response-size
/// limit" networking control.
const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Reads `response`'s body up to `max_bytes`, rejecting it as soon as
/// that limit is exceeded — checked against a declared `Content-Length`
/// up front where present, but enforced against the actual bytes
/// streamed regardless, since a server can omit or lie about that
/// header (e.g. chunked transfer-encoding).
async fn read_capped_body(
    mut response: reqwest::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    if let Some(len) = response.content_length() {
        if len > max_bytes {
            return Err(format!(
                "response declared {len} bytes, exceeding the {max_bytes}-byte limit"
            ));
        }
    }
    let mut buf = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > max_bytes {
            return Err(format!(
                "response exceeded the {max_bytes}-byte limit while streaming"
            ));
        }
    }
    Ok(buf)
}

/// Reads the feed URL a `Source` is configured with out of its
/// `configuration_json` — the only per-source state this connector
/// needs; it's otherwise stateless (see `trait_def.rs`'s doc comment).
fn feed_url(source: &Source) -> Option<String> {
    source
        .configuration_json
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[derive(Default)]
pub struct FeedConnector {
    client: reqwest::Client,
}

impl FeedConnector {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Connector for FeedConnector {
    fn identify(&self) -> &'static str {
        "RSS/Atom feed"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            search: false,
            browse: true,
            item_details: true,
            streaming: false,
            downloads: true,
            comments: false,
            authentication: vec![AuthMethod::Anonymous],
            media_types: vec![MediaType::Story, MediaType::Other],
            pagination: PaginationMode::None,
            rate_limit: None,
        }
    }

    async fn health_check(&self, source: &Source) -> ConnectorResult<HealthState> {
        let Some(url) = feed_url(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured feed url".to_string(),
            );
        };
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                ConnectorResult::Success(HealthState::Healthy)
            }
            Ok(_) => ConnectorResult::Success(HealthState::Degraded),
            Err(_) => ConnectorResult::Success(HealthState::Unreachable),
        }
    }

    async fn browse(
        &self,
        source: &Source,
        _page: Option<&str>,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        let Some(url) = feed_url(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured feed url".to_string(),
            );
        };

        let response = match self.client.get(&url).send().await {
            Ok(response) => response,
            Err(e) => return ConnectorResult::TemporaryFailure(e.to_string()),
        };
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return ConnectorResult::NotFound;
        }
        if !response.status().is_success() {
            return ConnectorResult::TemporaryFailure(format!(
                "unexpected response status: {}",
                response.status()
            ));
        }

        let bytes = match read_capped_body(response, MAX_RESPONSE_BYTES).await {
            Ok(bytes) => bytes,
            Err(msg) => return ConnectorResult::PermanentFailure(msg),
        };

        match feed_rs::parser::parse(&bytes[..]) {
            Ok(feed) => ConnectorResult::Success(
                feed.entries.into_iter().map(entry_to_remote_item).collect(),
            ),
            Err(e) => ConnectorResult::PermanentFailure(format!("could not parse feed: {e}")),
        }
    }

    async fn get_item(&self, source: &Source, source_item_id: &str) -> ConnectorResult<RemoteItem> {
        match self.browse(source, None).await {
            ConnectorResult::Success(items) | ConnectorResult::Partial(items) => {
                match items
                    .into_iter()
                    .find(|item| item.source_item_id == source_item_id)
                {
                    Some(item) => ConnectorResult::Success(item),
                    None => ConnectorResult::NotFound,
                }
            }
            ConnectorResult::AuthenticationRequired => ConnectorResult::AuthenticationRequired,
            ConnectorResult::RateLimited => ConnectorResult::RateLimited,
            ConnectorResult::UnsupportedQuery => ConnectorResult::UnsupportedQuery,
            ConnectorResult::UnsupportedCapability => ConnectorResult::UnsupportedCapability,
            ConnectorResult::NotFound => ConnectorResult::NotFound,
            ConnectorResult::Deleted => ConnectorResult::Deleted,
            ConnectorResult::BlockedBySource => ConnectorResult::BlockedBySource,
            ConnectorResult::TemporaryFailure(msg) => ConnectorResult::TemporaryFailure(msg),
            ConnectorResult::PermanentFailure(msg) => ConnectorResult::PermanentFailure(msg),
        }
    }
}

/// A downloadable attachment for an entry, if it has one — an RSS
/// `<enclosure>` (parsed by `feed_rs` into a default `MediaObject`'s
/// `content`, per its RSS2 parser) or an Atom `<link rel="enclosure">`.
/// RSS is checked first since it's the more common "podcast-style
/// attachment" shape; only the first eligible attachment is used —
/// this milestone downloads a single primary variant per item, not a
/// gallery of attachments.
fn entry_download(entry: &feed_rs::model::Entry) -> (Option<String>, Option<String>, Option<u64>) {
    for media in &entry.media {
        for content in &media.content {
            if let Some(url) = &content.url {
                return (
                    Some(url.to_string()),
                    content.content_type.as_ref().map(|m| m.to_string()),
                    content.size,
                );
            }
        }
    }
    for link in &entry.links {
        if link.rel.as_deref() == Some("enclosure") {
            return (
                Some(link.href.clone()),
                link.media_type.clone(),
                link.length,
            );
        }
    }
    (None, None, None)
}

fn entry_to_remote_item(entry: feed_rs::model::Entry) -> RemoteItem {
    let (download_url, download_mime_type, download_size_bytes) = entry_download(&entry);
    RemoteItem {
        source_item_id: entry.id,
        title: entry
            .title
            .map(|text| text.content)
            .unwrap_or_else(|| "(untitled)".to_string()),
        description: entry
            .summary
            .map(|text| text.content)
            .or_else(|| entry.content.and_then(|content| content.body)),
        canonical_url: entry.links.first().map(|link| link.href.clone()),
        tags: entry
            .categories
            .into_iter()
            .map(|category| category.term)
            .collect(),
        media_type: MediaType::Story,
        thumbnail_url: None,
        download_url,
        download_mime_type,
        download_size_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_with_url(url: &str) -> Source {
        Source {
            id: domain::SourceId::new(),
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

    #[test]
    fn capabilities_declare_downloads_but_not_search() {
        let capabilities = FeedConnector::new().capabilities();
        assert!(!capabilities.search, "feeds have no server-side search");
        assert!(
            capabilities.downloads,
            "the connector can fetch enclosure URLs, but per-entry \
             eligibility still depends on that specific entry having one"
        );
    }

    #[tokio::test]
    async fn browse_without_a_configured_url_is_a_permanent_failure() {
        let connector = FeedConnector::new();
        let source = source_with_url("");
        let mut source = source;
        source.configuration_json = serde_json::json!({});
        match connector.browse(&source, None).await {
            ConnectorResult::PermanentFailure(_) => {}
            other => panic!("expected PermanentFailure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn entry_conversion_falls_back_to_untitled_when_missing() {
        let entry = feed_rs::model::Entry {
            id: "guid-1".to_string(),
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(item.title, "(untitled)");
        assert_eq!(item.source_item_id, "guid-1");
    }

    #[tokio::test]
    async fn entry_conversion_extracts_an_rss_enclosure_as_the_download_url() {
        let media = feed_rs::model::MediaObject {
            content: vec![feed_rs::model::MediaContent {
                url: Some(url::Url::parse("http://example.test/episode.mp3").unwrap()),
                content_type: Some("audio/mpeg".parse().unwrap()),
                height: None,
                width: None,
                duration: None,
                size: Some(1234),
                rating: None,
            }],
            ..Default::default()
        };
        let entry = feed_rs::model::Entry {
            id: "guid-1".to_string(),
            media: vec![media],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(
            item.download_url.as_deref(),
            Some("http://example.test/episode.mp3")
        );
        assert_eq!(item.download_mime_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(item.download_size_bytes, Some(1234));
    }

    #[tokio::test]
    async fn entry_conversion_extracts_an_atom_enclosure_link_as_the_download_url() {
        let entry = feed_rs::model::Entry {
            id: "guid-1".to_string(),
            links: vec![
                feed_rs::model::Link {
                    href: "http://example.test/page".to_string(),
                    rel: Some("alternate".to_string()),
                    media_type: None,
                    href_lang: None,
                    title: None,
                    length: None,
                },
                feed_rs::model::Link {
                    href: "http://example.test/episode.mp3".to_string(),
                    rel: Some("enclosure".to_string()),
                    media_type: Some("audio/mpeg".to_string()),
                    href_lang: None,
                    title: None,
                    length: Some(5678),
                },
            ],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(
            item.download_url.as_deref(),
            Some("http://example.test/episode.mp3")
        );
        assert_eq!(item.download_mime_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(item.download_size_bytes, Some(5678));
    }

    #[tokio::test]
    async fn entry_conversion_has_no_download_url_without_an_enclosure() {
        let entry = feed_rs::model::Entry {
            id: "guid-1".to_string(),
            links: vec![feed_rs::model::Link {
                href: "http://example.test/page".to_string(),
                rel: Some("alternate".to_string()),
                media_type: None,
                href_lang: None,
                title: None,
                length: None,
            }],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(item.download_url, None);
    }
}
