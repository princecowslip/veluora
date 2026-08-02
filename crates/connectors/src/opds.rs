//! An OPDS (Open Publication Distribution System) connector — Workstream
//! 10's next connector after the RSS/Atom feed and booru connectors
//! (`docs/46-implementation-plan.md`), and one of `docs/41`'s catalogued
//! self-hosted book/comic/manga server APIs: Komga, Kavita, and
//! Calibre-Web all serve OPDS 1.x catalogs.
//!
//! OPDS 1.x is Atom+XML with a small set of extension links/attributes
//! layered on top, so this reuses `feed_rs` (already a dependency for
//! `feed.rs`) rather than adding a direct XML dependency — an entry is
//! classified as a navigation entry (pointing at a sub-catalog feed) or
//! an acquisition entry (pointing at a downloadable publication) purely
//! by inspecting its `<link>` `rel`/`type` attributes, both of which
//! `feed_rs::model::Link` already exposes.
//!
//! OPDS 2.0 (a JSON-based successor format) is out of scope for this
//! milestone — see `KNOWN_ISSUES.md`.
//!
//! Fully testable offline (see `tests/opds_connector.rs`, which serves
//! fixture catalog XML from a real local HTTP server rather than
//! depending on a live OPDS server).

use async_trait::async_trait;
use domain::{
    AuthMethod, ConnectorCapabilities, ConnectorId, ConnectorResult, HealthState, MediaType,
    PaginationMode, RemoteItem, Source,
};
use uuid::Uuid;

use crate::http_util::read_capped_body;
use crate::Connector;

/// Fixed so a `Source.connector_id` reliably identifies "this is the
/// OPDS connector" across restarts — connectors aren't looked up by
/// name at runtime, only by this stable id. `1` is the feed connector,
/// `2` is the booru connector (see `feed.rs`/`booru.rs`).
pub const OPDS_CONNECTOR_ID: ConnectorId = ConnectorId(Uuid::from_u128(3));

/// Same guard `feed.rs`/`booru.rs` use against a malicious or
/// misconfigured source exhausting memory.
const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// The `type` (`Link::media_type`) substring OPDS 1.2 uses on
/// navigation `<link>`s pointing at another catalog feed, e.g.
/// `application/atom+xml;profile=opds-catalog;kind=navigation`.
const OPDS_CATALOG_PROFILE: &str = "profile=opds-catalog";

/// The `rel` prefix OPDS uses on acquisition `<link>`s pointing at a
/// downloadable publication — a bare `http://opds-spec.org/acquisition`,
/// or a suffixed variant like `.../open-access` or `.../borrow`.
const OPDS_ACQUISITION_REL_PREFIX: &str = "http://opds-spec.org/acquisition";

struct OpdsConfig {
    url: String,
    username: Option<String>,
    password: Option<String>,
}

/// Reads a `Source`'s OPDS configuration — `url` is required,
/// `username`/`password` are optional (Komga, Kavita, and Calibre-Web
/// all typically sit behind HTTP Basic auth). Stored directly in
/// `configuration_json`, the same plain-config pattern `FeedConnector`'s
/// `url` and `BooruConnector`'s `api_key` already use — there is still
/// no OS-credential-manager adapter in this codebase.
fn opds_config(source: &Source) -> Option<OpdsConfig> {
    let url = source
        .configuration_json
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())?;
    Some(OpdsConfig {
        url,
        username: non_empty_str(source, "username"),
        password: non_empty_str(source, "password"),
    })
}

fn non_empty_str(source: &Source, key: &str) -> Option<String> {
    source
        .configuration_json
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

#[derive(Default)]
pub struct OpdsConnector {
    client: reqwest::Client,
}

impl OpdsConnector {
    pub fn new() -> Self {
        Self::default()
    }

    fn request(&self, config: &OpdsConfig, url: &str) -> reqwest::RequestBuilder {
        let request = self.client.get(url);
        match &config.username {
            Some(username) => request.basic_auth(username, config.password.as_ref()),
            None => request,
        }
    }

    /// Fetches and parses the catalog feed at `url` — used for both
    /// `browse` (the configured root, or a cursor from a previous page)
    /// and `get_gallery` (a sub-feed URL discovered from a prior
    /// navigation entry).
    async fn fetch_feed(&self, config: &OpdsConfig, url: &str) -> ConnectorResult<Vec<RemoteItem>> {
        let response = match self.request(config, url).send().await {
            Ok(response) => response,
            Err(e) => return ConnectorResult::TemporaryFailure(e.to_string()),
        };
        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            return ConnectorResult::AuthenticationRequired;
        }
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

        // OPDS catalogs commonly use relative `href`s (Komga/Kavita/
        // Calibre-Web all do) expected to resolve against the feed's own
        // URL — `base_uri` is exactly the hook `feed_rs` provides for
        // that, so entries' navigation/acquisition links come out as
        // absolute URLs regardless of how the server wrote them.
        let parser = feed_rs::parser::Builder::new().base_uri(Some(url)).build();
        match parser.parse(&bytes[..]) {
            Ok(feed) => ConnectorResult::Success(
                feed.entries.into_iter().map(entry_to_remote_item).collect(),
            ),
            Err(e) => ConnectorResult::PermanentFailure(format!("could not parse catalog: {e}")),
        }
    }
}

#[async_trait]
impl Connector for OpdsConnector {
    fn identify(&self) -> &'static str {
        "OPDS catalog"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            search: false,
            browse: true,
            item_details: true,
            streaming: false,
            downloads: true,
            comments: false,
            // Basic auth has no dedicated `AuthMethod` variant; grouped
            // under `ApiToken` alongside `BooruConnector`'s optional
            // `api_key`, matching that connector's precedent for "a
            // static, non-interactive credential" rather than a true
            // bearer token.
            authentication: vec![AuthMethod::Anonymous, AuthMethod::ApiToken],
            // Only the types this connector actually emits (see
            // `entry_to_remote_item`) — `Manga` is deliberately excluded
            // since OPDS gives no generic, connector-agnostic way to
            // distinguish manga from any other comic archive.
            media_types: vec![
                MediaType::Story,
                MediaType::Comic,
                MediaType::Gallery,
                MediaType::Other,
            ],
            pagination: PaginationMode::Cursor,
            rate_limit: None,
        }
    }

    async fn health_check(&self, source: &Source) -> ConnectorResult<HealthState> {
        let Some(config) = opds_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured OPDS catalog url".to_string(),
            );
        };
        let url = config.url.clone();
        match self.request(&config, &url).send().await {
            Ok(response) if response.status().is_success() => {
                ConnectorResult::Success(HealthState::Healthy)
            }
            Ok(_) => ConnectorResult::Success(HealthState::Degraded),
            Err(_) => ConnectorResult::Success(HealthState::Unreachable),
        }
    }

    /// `page: None` fetches the configured root catalog URL. `Some`
    /// cursors are opaque strings this connector itself produces — the
    /// href of the previous page's `rel="next"` feed-level link — not a
    /// caller-constructed value, matching `PaginationMode::Cursor`'s
    /// contract.
    async fn browse(
        &self,
        source: &Source,
        page: Option<&str>,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        let Some(config) = opds_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured OPDS catalog url".to_string(),
            );
        };
        let url = page
            .map(str::to_string)
            .unwrap_or_else(|| config.url.clone());
        self.fetch_feed(&config, &url).await
    }

    async fn get_item(&self, source: &Source, source_item_id: &str) -> ConnectorResult<RemoteItem> {
        // Only resolves ids present in the root catalog feed — an OPDS
        // hierarchy has no global id index, so an item nested inside a
        // navigation sub-feed (only reachable via a prior `get_gallery`
        // call) isn't found here. Same shape of limitation
        // `BooruConnector` documents for Gelbooru's missing by-id route.
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

    /// The first real exerciser of this trait method — neither
    /// `FeedConnector` nor `BooruConnector` implements it (see
    /// `KNOWN_ISSUES.md`). `source_item_id` here is a navigation entry's
    /// own `source_item_id` (see `entry_to_remote_item`), which this
    /// connector deliberately sets to the sub-feed's URL rather than the
    /// entry's Atom id — an OPDS catalog's hierarchy is discovered by
    /// walking `<link>`s, not by a global id lookup, so the URL doubles
    /// as both the item's identity and the fetch target.
    async fn get_gallery(
        &self,
        source: &Source,
        source_item_id: &str,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        let Some(config) = opds_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured OPDS catalog url".to_string(),
            );
        };
        self.fetch_feed(&config, source_item_id).await
    }
}

fn acquisition_link(links: &[feed_rs::model::Link]) -> Option<&feed_rs::model::Link> {
    links.iter().find(|link| {
        link.rel
            .as_deref()
            .is_some_and(|rel| rel.starts_with(OPDS_ACQUISITION_REL_PREFIX))
    })
}

fn navigation_link(links: &[feed_rs::model::Link]) -> Option<&feed_rs::model::Link> {
    links.iter().find(|link| {
        link.media_type
            .as_deref()
            .is_some_and(|media_type| media_type.contains(OPDS_CATALOG_PROFILE))
    })
}

fn alternate_link(links: &[feed_rs::model::Link]) -> Option<&feed_rs::model::Link> {
    links
        .iter()
        .find(|link| link.rel.as_deref() == Some("alternate"))
}

/// OPDS's thumbnail/cover-image link relations — `docs/41`-adjacent
/// browse UIs want a preview image without fetching the full
/// acquisition link.
fn thumbnail_link(links: &[feed_rs::model::Link]) -> Option<String> {
    links
        .iter()
        .find(|link| {
            link.rel.as_deref().is_some_and(|rel| {
                rel == "http://opds-spec.org/image/thumbnail" || rel == "http://opds-spec.org/image"
            })
        })
        .map(|link| link.href.clone())
}

/// Maps an acquisition link's declared mime type to the closest
/// `MediaType` this connector knows how to tag — best-effort, since OPDS
/// gives no dedicated "this is manga" signal distinct from any other
/// comic archive.
fn media_type_from_mime(mime: Option<&str>) -> MediaType {
    match mime {
        Some(m) if m.contains("epub") => MediaType::Story,
        Some(m) if m.starts_with("text/") => MediaType::Story,
        Some(m) if m.contains("comic") => MediaType::Comic,
        _ => MediaType::Other,
    }
}

fn entry_to_remote_item(entry: feed_rs::model::Entry) -> RemoteItem {
    let title = entry
        .title
        .map(|text| text.content)
        .unwrap_or_else(|| "(untitled)".to_string());
    let description = entry
        .summary
        .map(|text| text.content)
        .or_else(|| entry.content.and_then(|content| content.body));
    let tags: Vec<String> = entry
        .categories
        .iter()
        .map(|category| category.term.clone())
        .collect();
    let thumbnail_url = thumbnail_link(&entry.links);

    if let Some(acquisition) = acquisition_link(&entry.links) {
        let download_url = Some(acquisition.href.clone());
        let download_mime_type = acquisition.media_type.clone();
        let download_size_bytes = acquisition.length;
        let canonical_url = alternate_link(&entry.links)
            .map(|link| link.href.clone())
            .or_else(|| download_url.clone());
        return RemoteItem {
            source_item_id: entry.id,
            title,
            description,
            canonical_url,
            tags,
            media_type: media_type_from_mime(download_mime_type.as_deref()),
            thumbnail_url,
            download_url,
            download_mime_type,
            download_size_bytes,
        };
    }

    if let Some(navigation) = navigation_link(&entry.links) {
        let href = navigation.href.clone();
        return RemoteItem {
            source_item_id: href.clone(),
            title,
            description,
            canonical_url: Some(href),
            tags,
            media_type: MediaType::Gallery,
            thumbnail_url,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };
    }

    // Neither an acquisition nor a navigation link — an entry OPDS
    // technically permits but that carries nothing this connector can
    // act on (no file to download, no sub-feed to browse into).
    RemoteItem {
        source_item_id: entry.id,
        title,
        description,
        canonical_url: alternate_link(&entry.links).map(|link| link.href.clone()),
        tags,
        media_type: MediaType::Other,
        thumbnail_url,
        download_url: None,
        download_mime_type: None,
        download_size_bytes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_with_config(config: serde_json::Value) -> Source {
        Source {
            id: domain::SourceId::new(),
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

    fn link(rel: Option<&str>, media_type: Option<&str>, href: &str) -> feed_rs::model::Link {
        feed_rs::model::Link {
            href: href.to_string(),
            rel: rel.map(str::to_string),
            media_type: media_type.map(str::to_string),
            href_lang: None,
            title: None,
            length: None,
        }
    }

    #[test]
    fn capabilities_declare_browse_and_downloads_but_not_search() {
        let capabilities = OpdsConnector::new().capabilities();
        assert!(!capabilities.search, "OPDS 1.x has no generic search API");
        assert!(capabilities.browse);
        assert!(capabilities.downloads);
        assert!(!capabilities.media_types.contains(&MediaType::Manga));
    }

    #[tokio::test]
    async fn browse_without_a_configured_url_is_a_permanent_failure() {
        let connector = OpdsConnector::new();
        let source = source_with_config(serde_json::json!({}));
        match connector.browse(&source, None).await {
            ConnectorResult::PermanentFailure(_) => {}
            other => panic!("expected PermanentFailure, got {other:?}"),
        }
    }

    #[test]
    fn config_reads_url_and_optional_credentials() {
        let source = source_with_config(serde_json::json!({
            "url": "https://opds.example.test/catalog",
            "username": "reader",
            "password": "hunter2",
        }));
        let config = opds_config(&source).unwrap();
        assert_eq!(config.url, "https://opds.example.test/catalog");
        assert_eq!(config.username.as_deref(), Some("reader"));
        assert_eq!(config.password.as_deref(), Some("hunter2"));
    }

    #[test]
    fn config_rejects_an_empty_url() {
        let source = source_with_config(serde_json::json!({ "url": "" }));
        assert!(opds_config(&source).is_none());
    }

    #[test]
    fn entry_conversion_treats_an_acquisition_link_as_downloadable() {
        let entry = feed_rs::model::Entry {
            id: "urn:uuid:book-1".to_string(),
            links: vec![
                link(Some("alternate"), None, "https://opds.example.test/book/1"),
                link(
                    Some("http://opds-spec.org/acquisition"),
                    Some("application/epub+zip"),
                    "https://opds.example.test/book/1/download",
                ),
            ],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(item.source_item_id, "urn:uuid:book-1");
        assert_eq!(
            item.download_url.as_deref(),
            Some("https://opds.example.test/book/1/download")
        );
        assert_eq!(
            item.download_mime_type.as_deref(),
            Some("application/epub+zip")
        );
        assert_eq!(item.media_type, MediaType::Story);
        assert_eq!(
            item.canonical_url.as_deref(),
            Some("https://opds.example.test/book/1")
        );
    }

    #[test]
    fn entry_conversion_treats_a_navigation_link_as_a_browsable_gallery() {
        let entry = feed_rs::model::Entry {
            id: "urn:uuid:series-1".to_string(),
            links: vec![link(
                Some("subsection"),
                Some("application/atom+xml;profile=opds-catalog;kind=acquisition"),
                "https://opds.example.test/series/1",
            )],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        // The sub-feed URL, not the Atom entry id, becomes the item's
        // `source_item_id` — `get_gallery` fetches it directly.
        assert_eq!(item.source_item_id, "https://opds.example.test/series/1");
        assert_eq!(item.media_type, MediaType::Gallery);
        assert_eq!(item.download_url, None);
    }

    #[test]
    fn entry_conversion_prefers_acquisition_over_navigation_when_both_present() {
        let entry = feed_rs::model::Entry {
            id: "urn:uuid:mixed-1".to_string(),
            links: vec![
                link(
                    Some("subsection"),
                    Some("application/atom+xml;profile=opds-catalog;kind=acquisition"),
                    "https://opds.example.test/series/1",
                ),
                link(
                    Some("http://opds-spec.org/acquisition/open-access"),
                    Some("application/vnd.comicbook+zip"),
                    "https://opds.example.test/volume/1/download",
                ),
            ],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(item.source_item_id, "urn:uuid:mixed-1");
        assert_eq!(item.media_type, MediaType::Comic);
        assert_eq!(
            item.download_url.as_deref(),
            Some("https://opds.example.test/volume/1/download")
        );
    }

    #[test]
    fn entry_conversion_falls_back_to_untitled_and_other_without_any_recognized_link() {
        let entry = feed_rs::model::Entry {
            id: "urn:uuid:plain-1".to_string(),
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(item.title, "(untitled)");
        assert_eq!(item.media_type, MediaType::Other);
        assert_eq!(item.download_url, None);
    }

    #[test]
    fn entry_conversion_reads_a_thumbnail_link() {
        let entry = feed_rs::model::Entry {
            id: "urn:uuid:book-2".to_string(),
            links: vec![link(
                Some("http://opds-spec.org/image/thumbnail"),
                Some("image/jpeg"),
                "https://opds.example.test/book/2/cover.jpg",
            )],
            ..Default::default()
        };
        let item = entry_to_remote_item(entry);
        assert_eq!(
            item.thumbnail_url.as_deref(),
            Some("https://opds.example.test/book/2/cover.jpg")
        );
    }
}
