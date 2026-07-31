//! Configured connector-backed sources — `docs/14-source-connectors.md`.
//!
//! `Source`/`SourceReference` and the `sources`/`source_references`
//! tables existed since Milestone A as placeholder scaffolding
//! ("Connectors themselves are out of scope for Milestone A") with no
//! code touching them until now. This is that downstream point:
//! CRUD on `sources`, dispatch through `connectors::Connector`, and
//! materializing a browsed `RemoteItem` into the local library.

use std::sync::Arc;

use connectors::{Connector, ConnectorRegistry, FeedConnector, FEED_CONNECTOR_ID};
use domain::{
    AuthMethod, Clause, ConnectorCapabilities, ConnectorId, ConnectorResult, HealthState, ItemId,
    MediaType, PaginationMode, RemoteItem, SearchQuery, Source, SourceId, VariantId,
};
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::media_classification::media_type_to_str;
use crate::search::SearchService;
use crate::time_format::{from_rfc3339, to_rfc3339};

/// Whether import/browse/health-check dispatch through the generic
/// `Connector` trait or straight to `SearchService` — see
/// `SourceService::browse`'s doc comment.
fn is_local(source: &Source) -> bool {
    source.connector_id == LOCAL_FILESYSTEM_CONNECTOR_ID
}

/// Fixed so a `Source.connector_id` reliably identifies "this is the
/// local library" across restarts, matching `connectors::FEED_CONNECTOR_ID`'s
/// convention. Lives here (not in `connectors`) because the connector
/// implementation itself lives here — see the module doc comment.
pub const LOCAL_FILESYSTEM_CONNECTOR_ID: ConnectorId = ConnectorId(uuid::Uuid::from_u128(0));

/// What the browsed-and-locally-filtered result of [`SourceService::browse`]
/// looked like, including which query clauses (if any) the connector
/// itself didn't support and had to be applied host-side — the
/// "translation report" `docs/14`'s query-translation section asks for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseReport {
    pub result: ConnectorResult<Vec<RemoteItem>>,
    pub unsupported_clauses: Vec<String>,
}

pub struct SourceService;

impl SourceService {
    pub fn add(
        ctx: &AppContext,
        connector_id: ConnectorId,
        display_name: String,
        configuration_json: serde_json::Value,
    ) -> Result<Source> {
        let source = Source {
            id: SourceId::new(),
            connector_id,
            display_name,
            enabled: true,
            configuration_json,
            credential_ref: None,
            health_state: HealthState::Unknown,
            last_health_check: None,
            capability_snapshot_json: None,
        };
        ctx.db
            .connection()
            .execute(
                "INSERT INTO sources (id, connector_id, display_name, enabled, configuration_json, credential_ref, health_state, last_health_check, capability_snapshot_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    source.id.to_string(),
                    source.connector_id.to_string(),
                    source.display_name,
                    source.enabled as i64,
                    source.configuration_json.to_string(),
                    source.credential_ref,
                    health_state_to_str(source.health_state),
                    Option::<String>::None,
                    Option::<String>::None,
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(source)
    }

    pub fn list(ctx: &AppContext) -> Result<Vec<Source>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, connector_id, display_name, enabled, configuration_json, credential_ref, health_state, last_health_check, capability_snapshot_json
                 FROM sources ORDER BY display_name",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map([], row_to_source)
            .map_err(database::DatabaseError::from)?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(sources)
    }

    pub fn find_by_id(ctx: &AppContext, id: SourceId) -> Result<Option<Source>> {
        let conn = ctx.db.connection();
        match conn.query_row(
            "SELECT id, connector_id, display_name, enabled, configuration_json, credential_ref, health_state, last_health_check, capability_snapshot_json
             FROM sources WHERE id = ?1",
            params![id.to_string()],
            row_to_source,
        ) {
            Ok(source) => Ok(Some(source)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(database::DatabaseError::from(e).into()),
        }
    }

    pub fn set_enabled(ctx: &AppContext, id: SourceId, enabled: bool) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute(
                "UPDATE sources SET enabled = ?1 WHERE id = ?2",
                params![enabled as i64, id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("source {id}")));
        }
        Ok(())
    }

    /// Cascades to `source_references` via the FK's `ON DELETE CASCADE`
    /// (enforced — `Database::configure` turns `PRAGMA foreign_keys` on).
    /// `media_items`/`media_variants` created via
    /// [`Self::import_remote_item`] are left in place, matching how
    /// `LibraryRootService::remove` preserves items when a folder root
    /// is unregistered.
    pub fn remove(ctx: &AppContext, id: SourceId) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute("DELETE FROM sources WHERE id = ?1", params![id.to_string()])
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("source {id}")));
        }
        Ok(())
    }

    /// Resolves a source and its backing connector together — the same
    /// lookup `health_check`/`browse` already do inline, factored out
    /// so callers like `DownloadService` don't re-wire the connector
    /// registry themselves.
    pub fn connector_for(ctx: &AppContext, id: SourceId) -> Result<(Source, Arc<dyn Connector>)> {
        let source = Self::require(ctx, id)?;
        let connector = Self::registry().get(source.connector_id).ok_or_else(|| {
            AppError::InvalidPath(format!(
                "no connector registered for {}",
                source.connector_id
            ))
        })?;
        Ok((source, connector))
    }

    pub async fn health_check(ctx: &AppContext, id: SourceId) -> Result<HealthState> {
        let (source, connector) = Self::connector_for(ctx, id)?;

        let health = match connector.health_check(&source).await {
            ConnectorResult::Success(state) => state,
            _ => HealthState::Unknown,
        };

        ctx.db
            .connection()
            .execute(
                "UPDATE sources SET health_state = ?1, last_health_check = ?2 WHERE id = ?3",
                params![
                    health_state_to_str(health),
                    to_rfc3339(time::OffsetDateTime::now_utc()),
                    id.to_string(),
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(health)
    }

    /// Browses (or, if the connector declares `capabilities().search`,
    /// searches) a source, reporting which query clauses (if any)
    /// weren't understood by the connector and had to be filtered
    /// locally afterward — per `docs/14`'s query-translation model.
    ///
    /// The local-filesystem connector is special-cased: `Connector`'s
    /// methods only ever receive a `Source` row (see `trait_def.rs`'s
    /// doc comment — connectors are stateless), but querying the local
    /// library needs `AppContext`. Rather than stretching the trait
    /// with an application-only parameter every *other* connector has
    /// no use for, `SourceService` dispatches straight to
    /// `SearchService` for this one built-in case.
    pub async fn browse(
        ctx: &AppContext,
        id: SourceId,
        query: Option<&str>,
    ) -> Result<BrowseReport> {
        let source = Self::require(ctx, id)?;

        if is_local(&source) {
            return Self::browse_local(ctx, query);
        }

        let connector = Self::registry().get(source.connector_id).ok_or_else(|| {
            AppError::InvalidPath(format!(
                "no connector registered for {}",
                source.connector_id
            ))
        })?;
        let capabilities = connector.capabilities();

        let parsed = match query {
            Some(raw) if !raw.trim().is_empty() => Some(
                domain::parse_search_query(raw)
                    .map_err(|e| AppError::InvalidQuery(e.to_string()))?,
            ),
            _ => None,
        };

        if capabilities.search {
            let result = match &parsed {
                Some(q) => connector.search(&source, q).await,
                None => connector.browse(&source, None).await,
            };
            return Ok(BrowseReport {
                result,
                unsupported_clauses: Vec::new(),
            });
        }

        // The connector can't search — browse everything and filter
        // free text locally. Structured field filters aren't
        // backfilled (there's no local index of a connector's remote
        // items to filter against beyond what it just returned), so
        // they're reported as unsupported rather than silently
        // ignored.
        let raw_result = connector.browse(&source, None).await;
        let Some(query) = parsed else {
            return Ok(BrowseReport {
                result: raw_result,
                unsupported_clauses: Vec::new(),
            });
        };

        let (free_text, unsupported) = split_query(&query);
        let filtered = match raw_result {
            ConnectorResult::Success(items) => {
                ConnectorResult::Success(filter_locally(items, &free_text))
            }
            ConnectorResult::Partial(items) => {
                ConnectorResult::Partial(filter_locally(items, &free_text))
            }
            other => other,
        };
        Ok(BrowseReport {
            result: filtered,
            unsupported_clauses: unsupported,
        })
    }

    /// The local library already fully supports the query grammar
    /// (`SearchService` *is* the engine — there's no translation gap),
    /// so this simply forwards the raw query string and reports
    /// nothing as unsupported.
    fn browse_local(ctx: &AppContext, query: Option<&str>) -> Result<BrowseReport> {
        let results = SearchService::search(ctx, query.unwrap_or(""), 100, 0)?;
        let items = results
            .items
            .into_iter()
            .map(search_hit_to_remote_item)
            .collect();
        Ok(BrowseReport {
            result: ConnectorResult::Success(items),
            unsupported_clauses: Vec::new(),
        })
    }

    /// Materializes a browsed [`RemoteItem`] into `media_items` +
    /// `media_variants` (with `remote_url` set, `local_path` left
    /// `NULL` — both columns have existed since Milestone A) plus a
    /// `source_references` row linking the two. This is what the
    /// previously-unused `source_references` table is for.
    ///
    /// Refuses the local-filesystem source: its items are already
    /// local `media_items` rows (that's what `browse_local` reads
    /// from) — "importing" one would create a duplicate.
    pub fn import_remote_item(
        ctx: &AppContext,
        source_id: SourceId,
        remote_item: RemoteItem,
    ) -> Result<ItemId> {
        let (source, connector) = Self::connector_for(ctx, source_id)?;
        if is_local(&source) {
            return Err(AppError::InvalidPath(
                "cannot import from the local library — its items are already local".to_string(),
            ));
        }
        let downloads_capable = connector.capabilities().downloads;
        let fetch_url = remote_item
            .download_url
            .clone()
            .or_else(|| remote_item.canonical_url.clone());
        let download_permitted = downloads_capable && remote_item.download_url.is_some();
        let mime_type = remote_item
            .download_mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let item_id = ItemId::new();
        let variant_id = VariantId::new();
        let source_ref_id = domain::SourceRefId::new();
        let now = to_rfc3339(time::OffsetDateTime::now_utc());

        let conn = ctx.db.connection();
        conn.execute(
            "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
             VALUES (?1, ?2, ?3, 'unrated', ?4, ?4)",
            params![
                item_id.to_string(),
                media_type_to_str(remote_item.media_type),
                remote_item.title,
                now,
            ],
        )
        .map_err(database::DatabaseError::from)?;

        // `source_references` must exist before `media_variants` can
        // reference it via `source_ref_id` (an FK-enforced table, per
        // `Database::configure`'s `PRAGMA foreign_keys = ON`).
        conn.execute(
            "INSERT INTO source_references (id, item_id, source_id, source_item_id, canonical_url, original_title, original_description, original_tags, access_state, last_checked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'available', ?9)",
            params![
                source_ref_id.to_string(),
                item_id.to_string(),
                source_id.to_string(),
                remote_item.source_item_id,
                remote_item.canonical_url,
                remote_item.title,
                remote_item.description,
                serde_json::to_string(&remote_item.tags).unwrap_or_default(),
                now,
            ],
        )
        .map_err(database::DatabaseError::from)?;

        conn.execute(
            "INSERT INTO media_variants (id, item_id, source_ref_id, remote_url, mime_type, format, file_size, download_permitted)
             VALUES (?1, ?2, ?3, ?4, ?5, 'remote', ?6, ?7)",
            params![
                variant_id.to_string(),
                item_id.to_string(),
                source_ref_id.to_string(),
                fetch_url,
                mime_type,
                remote_item.download_size_bytes.map(|n| n as i64),
                download_permitted as i64,
            ],
        )
        .map_err(database::DatabaseError::from)?;

        Ok(item_id)
    }

    fn require(ctx: &AppContext, id: SourceId) -> Result<Source> {
        Self::find_by_id(ctx, id)?.ok_or_else(|| AppError::NotFound(format!("source {id}")))
    }

    /// A fresh registry per call — connectors are stateless singletons
    /// (see `connectors::trait_def`'s doc comment), so this is just two
    /// cheap allocations, not a real cache miss.
    fn registry() -> ConnectorRegistry {
        let mut registry = ConnectorRegistry::new();
        registry.register(FEED_CONNECTOR_ID, Arc::new(FeedConnector::new()));
        registry.register(
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            Arc::new(LocalFilesystemConnector),
        );
        registry
    }
}

/// Splits a parsed query into its free-text portion (joined with
/// spaces) and a human-readable list of the field-filter clauses that
/// can't be checked against a connector's `RemoteItem`s.
fn split_query(query: &SearchQuery) -> (String, Vec<String>) {
    let mut free_text_parts = Vec::new();
    let mut unsupported = Vec::new();
    for clause in &query.clauses {
        match clause {
            Clause::FreeText(text) => free_text_parts.push(text.clone()),
            Clause::Field(filter) => unsupported.push(format!("{:?}", filter.field)),
            Clause::Or(_) => unsupported.push("(grouped OR clause)".to_string()),
        }
    }
    (free_text_parts.join(" "), unsupported)
}

fn filter_locally(items: Vec<RemoteItem>, free_text: &str) -> Vec<RemoteItem> {
    if free_text.trim().is_empty() {
        return items;
    }
    let needle = free_text.to_lowercase();
    items
        .into_iter()
        .filter(|item| {
            item.title.to_lowercase().contains(&needle)
                || item
                    .description
                    .as_deref()
                    .is_some_and(|d| d.to_lowercase().contains(&needle))
                || item.tags.iter().any(|t| t.to_lowercase().contains(&needle))
        })
        .collect()
}

/// Wraps the already-tested, already-complete local library
/// (`SearchService`) behind the `Connector` trait — proves the
/// framework against code that already works, rather than retrofitting
/// the scanner to write `source_references` rows for every local file
/// (separate, riskier surgery not required by this workstream's
/// acceptance criteria).
struct LocalFilesystemConnector;

#[async_trait::async_trait]
impl Connector for LocalFilesystemConnector {
    fn identify(&self) -> &'static str {
        "Local library"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            search: true,
            browse: true,
            item_details: true,
            streaming: false,
            downloads: false,
            comments: false,
            authentication: vec![AuthMethod::Anonymous],
            media_types: vec![
                MediaType::Video,
                MediaType::Image,
                MediaType::Gallery,
                MediaType::Audio,
                MediaType::Story,
                MediaType::Manga,
                MediaType::Comic,
            ],
            pagination: PaginationMode::Offset,
            rate_limit: None,
        }
    }

    async fn health_check(&self, _source: &Source) -> ConnectorResult<HealthState> {
        // The local library is either reachable (the process is
        // running) or this code wouldn't be executing at all.
        ConnectorResult::Success(HealthState::Healthy)
    }

    // `search`/`browse` are never actually called through this trait
    // object for the local connector — `SourceService::browse`
    // dispatches straight to `SearchService` instead, since it needs
    // `AppContext` and the trait deliberately doesn't thread that
    // through (see `trait_def.rs`'s doc comment). This impl still
    // exists so `identify()`/`capabilities()`/`health_check()` work
    // uniformly across every registered connector, local included.
}

pub(crate) fn search_hit_to_remote_item(hit: crate::search::SearchHit) -> RemoteItem {
    RemoteItem {
        source_item_id: hit.item_id,
        title: hit.title,
        description: None,
        canonical_url: None,
        tags: Vec::new(),
        media_type: hit.media_type,
        thumbnail_url: None,
        download_url: None,
        download_mime_type: None,
        download_size_bytes: None,
    }
}

fn row_to_source(row: &Row) -> rusqlite::Result<Source> {
    let id: String = row.get(0)?;
    let connector_id: String = row.get(1)?;
    let configuration_json: String = row.get(4)?;
    let health_state: String = row.get(6)?;
    let last_health_check: Option<String> = row.get(7)?;
    let capability_snapshot_json: Option<String> = row.get(8)?;

    Ok(Source {
        id: SourceId(id.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(0, "id".into(), rusqlite::types::Type::Text)
        })?),
        connector_id: ConnectorId(connector_id.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                1,
                "connector_id".into(),
                rusqlite::types::Type::Text,
            )
        })?),
        display_name: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        configuration_json: serde_json::from_str(&configuration_json)
            .unwrap_or(serde_json::Value::Null),
        credential_ref: row.get(5)?,
        health_state: health_state_from_str(&health_state),
        last_health_check: last_health_check.and_then(|s| from_rfc3339(&s)),
        capability_snapshot_json: capability_snapshot_json
            .and_then(|s| serde_json::from_str(&s).ok()),
    })
}

fn health_state_to_str(state: HealthState) -> &'static str {
    match state {
        HealthState::Unknown => "unknown",
        HealthState::Healthy => "healthy",
        HealthState::Degraded => "degraded",
        HealthState::Unreachable => "unreachable",
    }
}

fn health_state_from_str(s: &str) -> HealthState {
    match s {
        "healthy" => HealthState::Healthy,
        "degraded" => HealthState::Degraded,
        "unreachable" => HealthState::Unreachable,
        _ => HealthState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors::FEED_CONNECTOR_ID;

    fn insert_media_item(ctx: &AppContext, title: &str, media_type: &str) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, ?2, ?3, 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string(), media_type, title],
            )
            .unwrap();
        item_id
    }

    #[test]
    fn add_list_find_and_remove_round_trip() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();

        assert_eq!(SourceService::list(&ctx).unwrap().len(), 1);
        assert_eq!(
            SourceService::find_by_id(&ctx, source.id)
                .unwrap()
                .unwrap()
                .display_name,
            "My Feed"
        );

        SourceService::remove(&ctx, source.id).unwrap();
        assert!(SourceService::find_by_id(&ctx, source.id)
            .unwrap()
            .is_none());
        assert!(SourceService::list(&ctx).unwrap().is_empty());
    }

    #[test]
    fn set_enabled_toggles_the_row() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({}),
        )
        .unwrap();
        assert!(source.enabled);

        SourceService::set_enabled(&ctx, source.id, false).unwrap();
        assert!(
            !SourceService::find_by_id(&ctx, source.id)
                .unwrap()
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn set_enabled_on_an_unknown_id_is_not_found() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = SourceService::set_enabled(&ctx, SourceId::new(), false).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn remove_on_an_unknown_id_is_not_found() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = SourceService::remove(&ctx, SourceId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn health_check_reports_healthy_for_the_local_connector() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local Library".to_string(),
            serde_json::json!({}),
        )
        .unwrap();

        let health = SourceService::health_check(&ctx, source.id).await.unwrap();
        assert_eq!(health, HealthState::Healthy);
        assert_eq!(
            SourceService::find_by_id(&ctx, source.id)
                .unwrap()
                .unwrap()
                .health_state,
            HealthState::Healthy
        );
    }

    #[tokio::test]
    async fn browse_local_forwards_to_search_service() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_media_item(&ctx, "A Video", "video");
        insert_media_item(&ctx, "A Story", "story");
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local Library".to_string(),
            serde_json::json!({}),
        )
        .unwrap();

        let report = SourceService::browse(&ctx, source.id, Some("type:video"))
            .await
            .unwrap();
        assert!(report.unsupported_clauses.is_empty());
        match report.result {
            ConnectorResult::Success(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].title, "A Video");
            }
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn browse_a_connector_with_no_search_capability_filters_free_text_locally_and_reports_field_filters_as_unsupported(
    ) {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "Missing Feed".to_string(),
            serde_json::json!({ "url": "http://127.0.0.1:1/does-not-exist.xml" }),
        )
        .unwrap();

        // A field filter the feed connector can never support, so it
        // must be reported rather than silently dropped.
        let report = SourceService::browse(&ctx, source.id, Some("type:video hello"))
            .await
            .unwrap();
        assert!(!report.unsupported_clauses.is_empty());
    }

    #[test]
    fn import_remote_item_creates_a_searchable_item_and_source_reference() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();

        let remote_item = domain::RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Imported Story".to_string(),
            description: Some("A description".to_string()),
            canonical_url: Some("https://example.test/story".to_string()),
            tags: vec!["fiction".to_string()],
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };

        let item_id = SourceService::import_remote_item(&ctx, source.id, remote_item).unwrap();

        let results = SearchService::search(&ctx, "Imported", 10, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].item_id, item_id.to_string());

        let source_ref_count: i64 = ctx
            .db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM source_references WHERE item_id = ?1",
                params![item_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_ref_count, 1);
    }

    #[test]
    fn import_remote_item_marks_the_variant_download_permitted_when_the_connector_and_entry_both_support_it(
    ) {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();

        let remote_item = domain::RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Episode One".to_string(),
            description: None,
            canonical_url: Some("https://example.test/episode-one".to_string()),
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: Some("https://example.test/files/episode-one.mp3".to_string()),
            download_mime_type: Some("audio/mpeg".to_string()),
            download_size_bytes: Some(654321),
        };

        let item_id = SourceService::import_remote_item(&ctx, source.id, remote_item).unwrap();

        let (remote_url, mime_type, file_size, download_permitted): (
            String,
            String,
            Option<i64>,
            i64,
        ) = ctx
            .db
            .connection()
            .query_row(
                "SELECT remote_url, mime_type, file_size, download_permitted FROM media_variants WHERE item_id = ?1",
                params![item_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(remote_url, "https://example.test/files/episode-one.mp3");
        assert_eq!(mime_type, "audio/mpeg");
        assert_eq!(file_size, Some(654321));
        assert_eq!(download_permitted, 1);
    }

    #[test]
    fn import_remote_item_on_the_local_connector_source_is_rejected() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local Library".to_string(),
            serde_json::json!({}),
        )
        .unwrap();

        let remote_item = domain::RemoteItem {
            source_item_id: "n/a".to_string(),
            title: "Should not import".to_string(),
            description: None,
            canonical_url: None,
            tags: Vec::new(),
            media_type: domain::MediaType::Other,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };

        let err = SourceService::import_remote_item(&ctx, source.id, remote_item).unwrap_err();
        assert!(matches!(err, AppError::InvalidPath(_)));
    }
}
