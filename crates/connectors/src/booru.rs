//! A generic Danbooru-API-compatible / Gelbooru-DAPI-compatible booru
//! connector — one of `docs/14`'s recommended reference connectors
//! ("generic booru-compatible API"), and Workstream 10's "One
//! Danbooru-family connector"/"One Gelbooru-family connector" line
//! items, covered by a single `flavor`-configured implementation
//! rather than two separate connector types.
//!
//! Read-only by design, matching `docs/41-expanded-source-catalogue.md`'s
//! explicit scope for booru sources: search, browse, tags, item
//! details, authorized viewing — no uploading, voting, commenting,
//! remote favourites, or moderation actions.
//!
//! Fully testable offline (see `tests/booru_connector.rs`, which serves
//! fixture JSON from a real local HTTP server rather than depending on
//! live internet access).

use std::time::{Duration, Instant};

use async_trait::async_trait;
use domain::{
    AuthMethod, Clause, ConnectorCapabilities, ConnectorId, ConnectorResult, FieldFilter,
    FilterValue, HealthState, MediaType, PaginationMode, Predicate, RateLimit, RemoteItem,
    SearchField, SearchQuery, Source,
};
use serde::Deserialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::http_util::read_capped_body;
use crate::Connector;

/// Fixed so a `Source.connector_id` reliably identifies "this is the
/// booru connector" across restarts — connectors aren't looked up by
/// name at runtime, only by this stable id. `0` is the local
/// filesystem connector, `1` is the feed connector (see `feed.rs`).
pub const BOORU_CONNECTOR_ID: ConnectorId = ConnectorId(Uuid::from_u128(2));

/// Same guard `feed.rs` uses against a malicious/misconfigured source
/// exhausting memory.
const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Posts per page for browse/search — a middle ground between too many
/// round trips and a response large enough to matter against the
/// oversized-response cap.
const DEFAULT_LIMIT: u32 = 40;

/// Advisory limit declared in `capabilities().rate_limit` and actually
/// enforced by `BooruConnector::throttle` — one request per second,
/// matching `docs/41`'s "conservative request limits" guidance for
/// booru sources.
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BooruFlavor {
    Danbooru,
    Gelbooru,
}

impl BooruFlavor {
    fn from_config_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "danbooru" => Some(Self::Danbooru),
            "gelbooru" => Some(Self::Gelbooru),
            _ => None,
        }
    }
}

struct BooruConfig {
    flavor: BooruFlavor,
    base_url: String,
    api_key: Option<String>,
    login_or_user_id: Option<String>,
}

/// Reads a `Source`'s booru configuration — `flavor` and `base_url` are
/// required, `api_key`/`login_or_user_id` are optional. The API key is
/// stored directly in `configuration_json`, the same pattern
/// `FeedConnector` uses for its URL — there is no OS-credential-manager
/// adapter in this codebase yet (`domain::Source.credential_ref` is
/// never set by any connector today).
fn booru_config(source: &Source) -> Option<BooruConfig> {
    let flavor = source
        .configuration_json
        .get("flavor")
        .and_then(|v| v.as_str())
        .and_then(BooruFlavor::from_config_str)?;
    let base_url = source
        .configuration_json
        .get("base_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())?;
    let api_key = non_empty_str(source, "api_key");
    let login_or_user_id = non_empty_str(source, "login_or_user_id");
    Some(BooruConfig {
        flavor,
        base_url,
        api_key,
        login_or_user_id,
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
pub struct BooruConnector {
    client: reqwest::Client,
    last_request: Mutex<Option<Instant>>,
}

impl BooruConnector {
    pub fn new() -> Self {
        Self::default()
    }

    /// A minimal self-throttle so the `rate_limit` this connector
    /// declares reflects real behavior rather than being purely
    /// documentary — see `docs/41`'s "conservative request limits"
    /// guidance for booru sources.
    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                tokio::time::sleep(MIN_REQUEST_INTERVAL - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }

    fn list_request(
        &self,
        config: &BooruConfig,
        tags: Option<&str>,
        page: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let page_num: u32 = page.and_then(|p| p.parse().ok()).unwrap_or(1).max(1);
        let base = &config.base_url;
        match config.flavor {
            BooruFlavor::Danbooru => {
                let mut params: Vec<(&str, String)> = vec![
                    ("limit", DEFAULT_LIMIT.to_string()),
                    ("page", page_num.to_string()),
                ];
                if let Some(tags) = tags.filter(|t| !t.is_empty()) {
                    params.push(("tags", tags.to_string()));
                }
                if let Some(key) = &config.api_key {
                    params.push(("api_key", key.clone()));
                }
                if let Some(login) = &config.login_or_user_id {
                    params.push(("login", login.clone()));
                }
                self.client.get(format!("{base}/posts.json")).query(&params)
            }
            BooruFlavor::Gelbooru => {
                let mut params: Vec<(&str, String)> = vec![
                    ("page", "dapi".to_string()),
                    ("s", "post".to_string()),
                    ("q", "index".to_string()),
                    ("json", "1".to_string()),
                    ("limit", DEFAULT_LIMIT.to_string()),
                    ("pid", (page_num - 1).to_string()),
                ];
                if let Some(tags) = tags.filter(|t| !t.is_empty()) {
                    params.push(("tags", tags.to_string()));
                }
                if let Some(key) = &config.api_key {
                    params.push(("api_key", key.clone()));
                }
                if let Some(uid) = &config.login_or_user_id {
                    params.push(("user_id", uid.clone()));
                }
                self.client.get(format!("{base}/index.php")).query(&params)
            }
        }
    }

    async fn fetch_posts(
        &self,
        config: &BooruConfig,
        tags: Option<&str>,
        page: Option<&str>,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        self.throttle().await;
        let response = match self.list_request(config, tags, page).send().await {
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

        match config.flavor {
            BooruFlavor::Danbooru => match serde_json::from_slice::<Vec<DanbooruPost>>(&bytes) {
                Ok(posts) => ConnectorResult::Success(
                    posts
                        .iter()
                        .map(|post| danbooru_post_to_remote_item(post, &config.base_url))
                        .collect(),
                ),
                Err(e) => ConnectorResult::PermanentFailure(format!("could not parse posts: {e}")),
            },
            BooruFlavor::Gelbooru => match parse_gelbooru_posts(&bytes) {
                Ok(posts) => ConnectorResult::Success(
                    posts
                        .iter()
                        .map(|post| gelbooru_post_to_remote_item(post, &config.base_url))
                        .collect(),
                ),
                Err(msg) => {
                    ConnectorResult::PermanentFailure(format!("could not parse posts: {msg}"))
                }
            },
        }
    }

    async fn fetch_single(&self, config: &BooruConfig, id: &str) -> ConnectorResult<RemoteItem> {
        match config.flavor {
            BooruFlavor::Danbooru => {
                self.throttle().await;
                let base = &config.base_url;
                let mut params: Vec<(&str, String)> = Vec::new();
                if let Some(key) = &config.api_key {
                    params.push(("api_key", key.clone()));
                }
                if let Some(login) = &config.login_or_user_id {
                    params.push(("login", login.clone()));
                }
                let response = match self
                    .client
                    .get(format!("{base}/posts/{id}.json"))
                    .query(&params)
                    .send()
                    .await
                {
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
                match serde_json::from_slice::<DanbooruPost>(&bytes) {
                    Ok(post) => ConnectorResult::Success(danbooru_post_to_remote_item(&post, base)),
                    Err(e) => {
                        ConnectorResult::PermanentFailure(format!("could not parse post: {e}"))
                    }
                }
            }
            // Gelbooru's DAPI has no dedicated by-id route — the
            // documented workaround is a search restricted to that id.
            BooruFlavor::Gelbooru => match self
                .fetch_posts(config, Some(&format!("id:{id}")), None)
                .await
            {
                ConnectorResult::Success(items) | ConnectorResult::Partial(items) => {
                    match items.into_iter().next() {
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
            },
        }
    }
}

#[async_trait]
impl Connector for BooruConnector {
    fn identify(&self) -> &'static str {
        "Booru (Danbooru/Gelbooru-compatible)"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            search: true,
            browse: true,
            item_details: true,
            streaming: false,
            downloads: true,
            comments: false,
            authentication: vec![AuthMethod::Anonymous, AuthMethod::ApiToken],
            media_types: vec![MediaType::Image, MediaType::Video],
            pagination: PaginationMode::Page,
            rate_limit: Some(RateLimit {
                requests: 60,
                period_seconds: 60,
            }),
        }
    }

    async fn health_check(&self, source: &Source) -> ConnectorResult<HealthState> {
        let Some(config) = booru_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured booru flavor and base_url".to_string(),
            );
        };
        self.throttle().await;
        match self.list_request(&config, None, Some("1")).send().await {
            Ok(response) if response.status().is_success() => {
                ConnectorResult::Success(HealthState::Healthy)
            }
            Ok(_) => ConnectorResult::Success(HealthState::Degraded),
            Err(_) => ConnectorResult::Success(HealthState::Unreachable),
        }
    }

    async fn search(
        &self,
        source: &Source,
        query: &SearchQuery,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        let Some(config) = booru_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured booru flavor and base_url".to_string(),
            );
        };
        let Some(tags) = translate_tags_query(query) else {
            return ConnectorResult::UnsupportedQuery;
        };
        self.fetch_posts(&config, Some(&tags), None).await
    }

    async fn browse(
        &self,
        source: &Source,
        page: Option<&str>,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        let Some(config) = booru_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured booru flavor and base_url".to_string(),
            );
        };
        self.fetch_posts(&config, None, page).await
    }

    async fn get_item(&self, source: &Source, source_item_id: &str) -> ConnectorResult<RemoteItem> {
        let Some(config) = booru_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured booru flavor and base_url".to_string(),
            );
        };
        self.fetch_single(&config, source_item_id).await
    }

    async fn get_tags(&self, source: &Source, prefix: &str) -> ConnectorResult<Vec<String>> {
        let Some(config) = booru_config(source) else {
            return ConnectorResult::PermanentFailure(
                "source has no configured booru flavor and base_url".to_string(),
            );
        };
        self.throttle().await;
        let base = &config.base_url;
        let request = match config.flavor {
            BooruFlavor::Danbooru => {
                let mut params: Vec<(&str, String)> = vec![
                    ("search[name_matches]", format!("{prefix}*")),
                    ("limit", "20".to_string()),
                ];
                if let Some(key) = &config.api_key {
                    params.push(("api_key", key.clone()));
                }
                self.client.get(format!("{base}/tags.json")).query(&params)
            }
            BooruFlavor::Gelbooru => {
                let mut params: Vec<(&str, String)> = vec![
                    ("page", "dapi".to_string()),
                    ("s", "tag".to_string()),
                    ("q", "index".to_string()),
                    ("json", "1".to_string()),
                    ("name_pattern", format!("{prefix}%")),
                ];
                if let Some(key) = &config.api_key {
                    params.push(("api_key", key.clone()));
                }
                self.client.get(format!("{base}/index.php")).query(&params)
            }
        };
        let response = match request.send().await {
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
        match config.flavor {
            BooruFlavor::Danbooru => match serde_json::from_slice::<Vec<DanbooruTag>>(&bytes) {
                Ok(tags) => ConnectorResult::Success(tags.into_iter().map(|t| t.name).collect()),
                Err(e) => ConnectorResult::PermanentFailure(format!("could not parse tags: {e}")),
            },
            BooruFlavor::Gelbooru => match parse_gelbooru_tags(&bytes) {
                Ok(tags) => ConnectorResult::Success(tags),
                Err(msg) => {
                    ConnectorResult::PermanentFailure(format!("could not parse tags: {msg}"))
                }
            },
        }
    }
}

/// Translates a parsed [`SearchQuery`] into the space-joined tag query
/// both APIs natively understand (`-tag` excludes). Only free text and
/// `tag:`/`​-tag:` equality clauses translate; anything else (any other
/// field, a comparison/range predicate, or an `(a OR b)` group) has no
/// honest translation, so the whole query is reported unsupported
/// rather than silently dropping part of it — see `booru.rs`'s module
/// doc and `KNOWN_ISSUES.md` for why this is coarser (all-or-nothing
/// per query) than the browse-only-connector local-filtering path.
fn translate_tags_query(query: &SearchQuery) -> Option<String> {
    let mut tags = Vec::new();
    for clause in &query.clauses {
        match clause {
            Clause::FreeText(text) => tags.push(text.clone()),
            Clause::Field(FieldFilter {
                field: SearchField::Tag,
                negated,
                predicate: Predicate::Equals(FilterValue::Text(value)),
            }) => {
                tags.push(if *negated {
                    format!("-{value}")
                } else {
                    value.clone()
                });
            }
            _ => return None,
        }
    }
    Some(tags.join(" "))
}

#[derive(Debug, Clone, Deserialize)]
struct DanbooruPost {
    id: i64,
    #[serde(default)]
    tag_string: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    file_url: Option<String>,
    #[serde(default)]
    large_file_url: Option<String>,
    #[serde(default)]
    preview_file_url: Option<String>,
    #[serde(default)]
    file_ext: Option<String>,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    rating: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DanbooruTag {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GelbooruPost {
    id: i64,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    file_url: Option<String>,
    #[serde(default)]
    sample_url: Option<String>,
    #[serde(default)]
    preview_url: Option<String>,
    #[serde(default)]
    rating: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GelbooruTag {
    name: String,
}

/// Gelbooru's DAPI (depending on installation/version) returns either a
/// bare JSON array, `{"post": [...]}`, a single wrapped object when
/// there's exactly one hit, or `{"post": null}`/no `post` key at all
/// for zero results — this normalizes all four shapes rather than
/// assuming any one of them.
fn parse_gelbooru_posts(bytes: &[u8]) -> Result<Vec<GelbooruPost>, String> {
    serde_json::from_value(normalize_envelope(bytes, "post")?).map_err(|e| e.to_string())
}

fn parse_gelbooru_tags(bytes: &[u8]) -> Result<Vec<String>, String> {
    let tags: Vec<GelbooruTag> =
        serde_json::from_value(normalize_envelope(bytes, "tag")?).map_err(|e| e.to_string())?;
    Ok(tags.into_iter().map(|t| t.name).collect())
}

fn normalize_envelope(bytes: &[u8], key: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let list = match &value {
        serde_json::Value::Array(_) => value,
        serde_json::Value::Object(map) => match map.get(key) {
            None | Some(serde_json::Value::Null) => serde_json::Value::Array(Vec::new()),
            Some(serde_json::Value::Array(items)) => serde_json::Value::Array(items.clone()),
            Some(single) => serde_json::Value::Array(vec![single.clone()]),
        },
        _ => return Err("expected a JSON array or object".to_string()),
    };
    Ok(list)
}

fn danbooru_post_to_remote_item(post: &DanbooruPost, base_url: &str) -> RemoteItem {
    let download_url = post
        .file_url
        .clone()
        .or_else(|| post.large_file_url.clone());
    let extension = post
        .file_ext
        .clone()
        .map(|ext| ext.to_ascii_lowercase())
        .or_else(|| download_url.as_deref().and_then(extension_from_url));
    let mut tags: Vec<String> = post
        .tag_string
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if let Some(rating) = &post.rating {
        tags.push(format!("rating:{}", normalize_rating(rating)));
    }
    RemoteItem {
        source_item_id: post.id.to_string(),
        title: format!("Post #{}", post.id),
        description: post.source.clone().filter(|s| !s.is_empty()),
        canonical_url: Some(format!("{base_url}/posts/{}", post.id)),
        tags,
        media_type: extension
            .as_deref()
            .map(media_type_from_extension)
            .unwrap_or(MediaType::Image),
        thumbnail_url: post.preview_file_url.clone(),
        download_url,
        download_mime_type: extension.as_deref().and_then(mime_from_extension),
        download_size_bytes: post.file_size,
    }
}

fn gelbooru_post_to_remote_item(post: &GelbooruPost, base_url: &str) -> RemoteItem {
    let download_url = post.file_url.clone().or_else(|| post.sample_url.clone());
    let extension = download_url.as_deref().and_then(extension_from_url);
    let mut tags: Vec<String> = post
        .tags
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if let Some(rating) = &post.rating {
        tags.push(format!("rating:{}", normalize_rating(rating)));
    }
    RemoteItem {
        source_item_id: post.id.to_string(),
        title: format!("Post #{}", post.id),
        description: post.source.clone().filter(|s| !s.is_empty()),
        canonical_url: Some(format!(
            "{base_url}/index.php?page=post&s=view&id={}",
            post.id
        )),
        tags,
        media_type: extension
            .as_deref()
            .map(media_type_from_extension)
            .unwrap_or(MediaType::Image),
        thumbnail_url: post.preview_url.clone(),
        download_url,
        // Gelbooru's DAPI post-list response has no file-size field.
        download_mime_type: extension.as_deref().and_then(mime_from_extension),
        download_size_bytes: None,
    }
}

/// Danbooru uses single-letter rating codes (`g`/`s`/`q`/`e`); Gelbooru
/// uses full words (`safe`/`questionable`/`explicit`). Normalized here
/// so a downstream reader sees one consistent vocabulary regardless of
/// flavor.
fn normalize_rating(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "g" | "safe" => "general",
        "s" | "sensitive" => "sensitive",
        "q" | "questionable" => "questionable",
        "e" | "explicit" => "explicit",
        other => return other.to_string(),
    }
    .to_string()
}

fn extension_from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let filename = path.rsplit('/').next()?;
    filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
}

fn media_type_from_extension(ext: &str) -> MediaType {
    match ext {
        "webm" | "mp4" | "mov" | "avi" | "wmv" | "mkv" => MediaType::Video,
        _ => MediaType::Image,
    }
}

fn mime_from_extension(ext: &str) -> Option<String> {
    Some(
        match ext {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "webm" => "video/webm",
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            _ => return None,
        }
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::SourceId;

    fn source_with_config(config: serde_json::Value) -> Source {
        Source {
            id: SourceId::new(),
            connector_id: BOORU_CONNECTOR_ID,
            display_name: "Test Booru".to_string(),
            enabled: true,
            configuration_json: config,
            credential_ref: None,
            health_state: HealthState::Unknown,
            last_health_check: None,
            capability_snapshot_json: None,
        }
    }

    #[test]
    fn capabilities_declare_search_browse_and_downloads() {
        let capabilities = BooruConnector::new().capabilities();
        assert!(
            capabilities.search,
            "both APIs support server-side tag search"
        );
        assert!(capabilities.browse);
        assert!(
            capabilities.downloads,
            "post file_url/sample_url fields are direct fetchable files"
        );
        assert!(!capabilities.comments, "explicitly excluded per docs/41");
    }

    #[tokio::test]
    async fn browse_without_a_configured_flavor_is_a_permanent_failure() {
        let connector = BooruConnector::new();
        let source = source_with_config(serde_json::json!({ "base_url": "https://example.test" }));
        match connector.browse(&source, None).await {
            ConnectorResult::PermanentFailure(_) => {}
            other => panic!("expected PermanentFailure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn browse_without_a_configured_base_url_is_a_permanent_failure() {
        let connector = BooruConnector::new();
        let source = source_with_config(serde_json::json!({ "flavor": "danbooru" }));
        match connector.browse(&source, None).await {
            ConnectorResult::PermanentFailure(_) => {}
            other => panic!("expected PermanentFailure, got {other:?}"),
        }
    }

    #[test]
    fn config_parses_flavor_case_insensitively_and_trims_trailing_slash() {
        let source = source_with_config(serde_json::json!({
            "flavor": "DANBOORU",
            "base_url": "https://danbooru.example.test/",
        }));
        let config = booru_config(&source).unwrap();
        assert_eq!(config.flavor, BooruFlavor::Danbooru);
        assert_eq!(config.base_url, "https://danbooru.example.test");
    }

    #[test]
    fn config_rejects_an_unknown_flavor() {
        let source = source_with_config(serde_json::json!({
            "flavor": "not-a-real-booru",
            "base_url": "https://example.test",
        }));
        assert!(booru_config(&source).is_none());
    }

    #[test]
    fn translate_tags_query_joins_free_text_and_tag_clauses() {
        let query = domain::parse_search_query("blue_eyes -tag:blocked good_boy").unwrap();
        let translated = translate_tags_query(&query).unwrap();
        assert_eq!(translated, "blue_eyes -blocked good_boy");
    }

    #[test]
    fn translate_tags_query_rejects_a_field_it_cannot_translate() {
        let query = domain::parse_search_query("width:>1920").unwrap();
        assert!(translate_tags_query(&query).is_none());
    }

    #[test]
    fn translate_tags_query_rejects_an_or_group() {
        let query = domain::parse_search_query("(cat OR dog)").unwrap();
        assert!(translate_tags_query(&query).is_none());
    }

    #[test]
    fn translate_tags_query_rejects_a_non_equals_tag_predicate() {
        // The real parser never produces a non-`Equals` predicate for
        // `Tag` (a `Text`-kind field, which `parse_predicate` never
        // treats as supporting `<`/`>`/`..` operators) — this
        // constructs the AST directly to exercise the connector's own
        // defensive handling of that case regardless.
        let query = SearchQuery {
            clauses: vec![Clause::Field(FieldFilter {
                field: SearchField::Tag,
                negated: false,
                predicate: Predicate::LessThan(FilterValue::Text("b".to_string())),
            })],
        };
        assert!(translate_tags_query(&query).is_none());
    }

    #[test]
    fn danbooru_post_conversion_maps_fields_and_folds_rating_into_tags() {
        let post = DanbooruPost {
            id: 42,
            tag_string: Some("blue_eyes 1girl".to_string()),
            source: Some("https://example.test/original".to_string()),
            file_url: Some("https://cdn.example.test/42.jpg".to_string()),
            large_file_url: None,
            preview_file_url: Some("https://cdn.example.test/42_preview.jpg".to_string()),
            file_ext: Some("jpg".to_string()),
            file_size: Some(123_456),
            rating: Some("e".to_string()),
        };
        let item = danbooru_post_to_remote_item(&post, "https://danbooru.example.test");
        assert_eq!(item.source_item_id, "42");
        assert_eq!(item.title, "Post #42");
        assert_eq!(
            item.canonical_url.as_deref(),
            Some("https://danbooru.example.test/posts/42")
        );
        assert_eq!(item.tags, vec!["blue_eyes", "1girl", "rating:explicit"]);
        assert_eq!(item.media_type, MediaType::Image);
        assert_eq!(
            item.download_url.as_deref(),
            Some("https://cdn.example.test/42.jpg")
        );
        assert_eq!(item.download_mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(item.download_size_bytes, Some(123_456));
    }

    #[test]
    fn danbooru_post_conversion_detects_video_by_extension() {
        let post = DanbooruPost {
            id: 7,
            tag_string: None,
            source: None,
            file_url: Some("https://cdn.example.test/7.webm".to_string()),
            large_file_url: None,
            preview_file_url: None,
            file_ext: None,
            file_size: None,
            rating: None,
        };
        let item = danbooru_post_to_remote_item(&post, "https://danbooru.example.test");
        assert_eq!(item.media_type, MediaType::Video);
        assert_eq!(item.download_mime_type.as_deref(), Some("video/webm"));
    }

    #[test]
    fn gelbooru_post_conversion_maps_fields() {
        let post = GelbooruPost {
            id: 99,
            tags: Some("cat cute".to_string()),
            source: None,
            file_url: Some("https://cdn.example.test/99.png".to_string()),
            sample_url: None,
            preview_url: Some("https://cdn.example.test/99_preview.png".to_string()),
            rating: Some("safe".to_string()),
        };
        let item = gelbooru_post_to_remote_item(&post, "https://gelbooru.example.test");
        assert_eq!(item.source_item_id, "99");
        assert_eq!(
            item.canonical_url.as_deref(),
            Some("https://gelbooru.example.test/index.php?page=post&s=view&id=99")
        );
        assert_eq!(item.tags, vec!["cat", "cute", "rating:general"]);
        assert_eq!(item.download_size_bytes, None);
    }

    #[test]
    fn parse_gelbooru_posts_handles_a_bare_array() {
        let body = br#"[{"id": 1, "tags": "a"}, {"id": 2, "tags": "b"}]"#;
        let posts = parse_gelbooru_posts(body).unwrap();
        assert_eq!(posts.len(), 2);
    }

    #[test]
    fn parse_gelbooru_posts_handles_a_wrapped_array() {
        let body = br#"{"post": [{"id": 1, "tags": "a"}]}"#;
        let posts = parse_gelbooru_posts(body).unwrap();
        assert_eq!(posts.len(), 1);
    }

    #[test]
    fn parse_gelbooru_posts_handles_a_single_wrapped_object() {
        let body = br#"{"post": {"id": 1, "tags": "a"}}"#;
        let posts = parse_gelbooru_posts(body).unwrap();
        assert_eq!(posts.len(), 1);
    }

    #[test]
    fn parse_gelbooru_posts_handles_a_null_post_key_as_empty() {
        let body = br#"{"post": null}"#;
        let posts = parse_gelbooru_posts(body).unwrap();
        assert!(posts.is_empty());
    }

    #[test]
    fn parse_gelbooru_posts_handles_a_missing_post_key_as_empty() {
        let body = br#"{}"#;
        let posts = parse_gelbooru_posts(body).unwrap();
        assert!(posts.is_empty());
    }
}
