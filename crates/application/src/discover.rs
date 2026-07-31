//! Unified cross-source search ("Discover") — Milestone I.
//!
//! Aggregates the local library (always searched, regardless of
//! whether a "Local filesystem" `Source` row has been explicitly
//! added) with every enabled, non-local connector source in one call.
//! Closes the gap `KNOWN_ISSUES.md` flagged three times, across the
//! Connectors, TUI, and GUI sections: `SearchService::search` only
//! ever covered the local library, and reaching a connector-backed
//! source required a separate, explicit `SourceService::browse` call
//! per source.

use domain::{ConnectorResult, ItemId, RemoteItem, Source, SourceId};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::search::SearchService;
use crate::source::{search_hit_to_remote_item, SourceService, LOCAL_FILESYSTEM_CONNECTOR_ID};

/// Stands in for the local library when no "Local filesystem" `Source`
/// row has been explicitly added — the local library is discoverable
/// unconditionally (see [`DiscoverService::discover`]), so it needs an
/// id to report even without a real row. Fixed for the same reason
/// `LOCAL_FILESYSTEM_CONNECTOR_ID` is fixed: stability across restarts.
/// When a real local-filesystem `Source` row does exist, its own id is
/// used instead — this sentinel is only the fallback.
pub const LOCAL_LIBRARY_SOURCE_ID: SourceId = SourceId(uuid::Uuid::from_u128(0));

/// One aggregated result: a [`RemoteItem`] plus which source produced
/// it, and — if it's already present in the local library — that
/// item's [`ItemId`]. Every UI's "Import" affordance is gated on
/// whether this is `Some`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverHit {
    pub source_id: SourceId,
    pub source_display_name: String,
    pub item: RemoteItem,
    pub local_item_id: Option<ItemId>,
}

/// Per-source outcome, kept separate from the flattened `hits` so one
/// broken connector's failure or unsupported-clause note isn't lost
/// among everything that worked — the concrete shape behind
/// `docs/46-implementation-plan.md` Workstream 10's "connector
/// failures are isolated" acceptance criterion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverSourceStatus {
    pub source_id: SourceId,
    pub source_display_name: String,
    /// The hit count this source contributed, wrapped in
    /// `ConnectorResult` to reuse its already-tested serde shape
    /// rather than inventing a parallel status enum.
    pub status: ConnectorResult<usize>,
    pub unsupported_clauses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverReport {
    pub schema_version: u32,
    pub query: String,
    pub hits: Vec<DiscoverHit>,
    pub sources: Vec<DiscoverSourceStatus>,
}

pub struct DiscoverService;

impl DiscoverService {
    /// `source_ids`: `None` fans out to every enabled non-local
    /// source; `Some(ids)` restricts fan-out to that set. Either way
    /// the local library is always searched. `limit_per_source` caps
    /// each source's contribution independently — there is no
    /// aggregate offset/next-page token across heterogeneous sources
    /// (local search has real limit/offset, `FeedConnector` takes an
    /// opaque cursor, other connectors may use yet another pagination
    /// mode), so Discover always returns "the first N per source" —
    /// see `KNOWN_ISSUES.md` for this documented as a known
    /// limitation rather than a unified pager.
    pub async fn discover(
        ctx: &AppContext,
        raw_query: &str,
        source_ids: Option<&[SourceId]>,
        limit_per_source: u32,
    ) -> Result<DiscoverReport> {
        // Fail fast on a malformed query before touching any
        // connector. The parsed AST itself is discarded —
        // `SearchService::search` and each `SourceService::browse`
        // call below reparse the raw string anyway, since that's the
        // interface they already have.
        domain::parse_search_query(raw_query).map_err(|e| AppError::InvalidQuery(e.to_string()))?;

        let all_sources = SourceService::list(ctx)?;
        let (local_source_id, local_source_display_name) = all_sources
            .iter()
            .find(|s| s.connector_id == LOCAL_FILESYSTEM_CONNECTOR_ID)
            .map(|s| (s.id, s.display_name.clone()))
            .unwrap_or((LOCAL_LIBRARY_SOURCE_ID, "Local library".to_string()));

        let mut hits = Vec::new();
        let mut sources = Vec::new();

        let local_results = SearchService::search(ctx, raw_query, limit_per_source, 0)?;
        let local_count = local_results.items.len();
        for search_hit in local_results.items {
            let item = search_hit_to_remote_item(search_hit);
            let local_item_id = uuid::Uuid::parse_str(&item.source_item_id).ok().map(ItemId);
            hits.push(DiscoverHit {
                source_id: local_source_id,
                source_display_name: local_source_display_name.clone(),
                item,
                local_item_id,
            });
        }
        sources.push(DiscoverSourceStatus {
            source_id: local_source_id,
            source_display_name: local_source_display_name.clone(),
            status: ConnectorResult::Success(local_count),
            unsupported_clauses: Vec::new(),
        });

        let candidate_sources: Vec<Source> = all_sources
            .into_iter()
            .filter(|s| s.enabled && s.connector_id != LOCAL_FILESYSTEM_CONNECTOR_ID)
            .filter(|s| source_ids.is_none_or(|ids| ids.contains(&s.id)))
            .collect();

        let browse_futures = candidate_sources
            .iter()
            .map(|source| SourceService::browse(ctx, source.id, Some(raw_query)));
        let browse_results = futures::future::join_all(browse_futures).await;

        for (source, browse_result) in candidate_sources.iter().zip(browse_results) {
            match browse_result {
                Ok(report) => {
                    let capped_result = match report.result {
                        ConnectorResult::Success(items) => {
                            ConnectorResult::Success(take_up_to(items, limit_per_source))
                        }
                        ConnectorResult::Partial(items) => {
                            ConnectorResult::Partial(take_up_to(items, limit_per_source))
                        }
                        other => other,
                    };
                    let (status, items) = split_status(capped_result);
                    for item in items {
                        let local_item_id =
                            find_linked_item_id(ctx, source.id, &item.source_item_id)?;
                        hits.push(DiscoverHit {
                            source_id: source.id,
                            source_display_name: source.display_name.clone(),
                            item,
                            local_item_id,
                        });
                    }
                    sources.push(DiscoverSourceStatus {
                        source_id: source.id,
                        source_display_name: source.display_name.clone(),
                        status,
                        unsupported_clauses: report.unsupported_clauses,
                    });
                }
                // A per-source service error (e.g. the source vanished
                // mid-call) doesn't abort the whole aggregate — it's
                // reported here, exactly like a connector-level
                // failure is reported via `status` above.
                Err(e) => sources.push(DiscoverSourceStatus {
                    source_id: source.id,
                    source_display_name: source.display_name.clone(),
                    status: ConnectorResult::PermanentFailure(e.to_string()),
                    unsupported_clauses: Vec::new(),
                }),
            }
        }

        Ok(DiscoverReport {
            schema_version: 1,
            query: raw_query.to_string(),
            hits,
            sources,
        })
    }
}

fn take_up_to(items: Vec<RemoteItem>, limit: u32) -> Vec<RemoteItem> {
    items.into_iter().take(limit as usize).collect()
}

/// Splits a [`ConnectorResult<Vec<RemoteItem>>`] into a
/// `ConnectorResult<usize>` (the hit count, for [`DiscoverSourceStatus`])
/// and the items themselves (for building [`DiscoverHit`]s) — only the
/// two payload-carrying variants have items to hand back.
fn split_status(
    result: ConnectorResult<Vec<RemoteItem>>,
) -> (ConnectorResult<usize>, Vec<RemoteItem>) {
    match result {
        ConnectorResult::Success(items) => (ConnectorResult::Success(items.len()), items),
        ConnectorResult::Partial(items) => (ConnectorResult::Partial(items.len()), items),
        ConnectorResult::AuthenticationRequired => {
            (ConnectorResult::AuthenticationRequired, Vec::new())
        }
        ConnectorResult::RateLimited => (ConnectorResult::RateLimited, Vec::new()),
        ConnectorResult::UnsupportedQuery => (ConnectorResult::UnsupportedQuery, Vec::new()),
        ConnectorResult::UnsupportedCapability => {
            (ConnectorResult::UnsupportedCapability, Vec::new())
        }
        ConnectorResult::NotFound => (ConnectorResult::NotFound, Vec::new()),
        ConnectorResult::Deleted => (ConnectorResult::Deleted, Vec::new()),
        ConnectorResult::BlockedBySource => (ConnectorResult::BlockedBySource, Vec::new()),
        ConnectorResult::TemporaryFailure(msg) => {
            (ConnectorResult::TemporaryFailure(msg), Vec::new())
        }
        ConnectorResult::PermanentFailure(msg) => {
            (ConnectorResult::PermanentFailure(msg), Vec::new())
        }
    }
}

/// Looks up whether a remote item has already been pulled into the
/// local library via a prior [`SourceService::import_remote_item`]
/// call, by matching `source_references` on `(source_id,
/// source_item_id)`. No repository abstraction exists in this
/// codebase — every service issues raw SQL directly (see
/// `SourceService`'s own queries) — so this is a second reader
/// alongside `import_remote_item`'s writer, not a new pattern.
fn find_linked_item_id(
    ctx: &AppContext,
    source_id: SourceId,
    source_item_id: &str,
) -> Result<Option<ItemId>> {
    let conn = ctx.db.connection();
    match conn.query_row(
        "SELECT item_id FROM source_references
         WHERE source_id = ?1 AND source_item_id = ?2 AND deleted_at IS NULL",
        params![source_id.to_string(), source_item_id],
        |row| row.get::<_, String>(0),
    ) {
        Ok(id) => {
            let uuid = uuid::Uuid::parse_str(&id).map_err(|e| {
                AppError::InvalidPath(format!("corrupt item_id in source_references: {e}"))
            })?;
            Ok(Some(ItemId(uuid)))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(database::DatabaseError::from(e).into()),
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

    #[tokio::test]
    async fn local_library_is_searched_with_no_source_rows_configured() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_media_item(&ctx, "A Video", "video");

        let report = DiscoverService::discover(&ctx, "type:video", None, 25)
            .await
            .unwrap();

        assert_eq!(report.hits.len(), 1);
        assert_eq!(report.hits[0].source_id, LOCAL_LIBRARY_SOURCE_ID);
        assert_eq!(report.hits[0].item.title, "A Video");
        assert!(report.hits[0].local_item_id.is_some());
        assert_eq!(report.sources.len(), 1);
    }

    #[tokio::test]
    async fn an_enabled_local_filesystem_source_row_is_not_duplicated() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_media_item(&ctx, "A Video", "video");
        SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local Library".to_string(),
            serde_json::json!({}),
        )
        .unwrap();

        let report = DiscoverService::discover(&ctx, "type:video", None, 25)
            .await
            .unwrap();

        assert_eq!(report.sources.len(), 1);
        assert_eq!(report.sources[0].source_display_name, "Local Library");
    }

    #[tokio::test]
    async fn source_ids_filter_restricts_remote_fan_out_but_local_is_always_included() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_media_item(&ctx, "A Video", "video");
        let feed = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "http://127.0.0.1:1/does-not-exist.xml" }),
        )
        .unwrap();
        let other_feed = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "Other Feed".to_string(),
            serde_json::json!({ "url": "http://127.0.0.1:1/does-not-exist.xml" }),
        )
        .unwrap();

        let report = DiscoverService::discover(&ctx, "video", Some(&[feed.id]), 25)
            .await
            .unwrap();

        let reported_ids: Vec<_> = report.sources.iter().map(|s| s.source_id).collect();
        assert!(reported_ids.contains(&LOCAL_LIBRARY_SOURCE_ID));
        assert!(reported_ids.contains(&feed.id));
        assert!(!reported_ids.contains(&other_feed.id));
    }

    #[tokio::test]
    async fn unsupported_clauses_are_surfaced_per_source() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "Missing Feed".to_string(),
            serde_json::json!({ "url": "http://127.0.0.1:1/does-not-exist.xml" }),
        )
        .unwrap();

        let report = DiscoverService::discover(&ctx, "type:video hello", None, 25)
            .await
            .unwrap();

        let status = report
            .sources
            .iter()
            .find(|s| s.source_id == source.id)
            .unwrap();
        assert!(!status.unsupported_clauses.is_empty());
    }

    #[tokio::test]
    async fn a_previously_imported_item_reports_its_local_item_id() {
        let ctx = AppContext::open_in_memory().unwrap();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "http://127.0.0.1:1/does-not-exist.xml" }),
        )
        .unwrap();
        let remote_item = domain::RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Imported Story".to_string(),
            description: None,
            canonical_url: None,
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };
        let item_id = SourceService::import_remote_item(&ctx, source.id, remote_item).unwrap();

        let linked = find_linked_item_id(&ctx, source.id, "guid-1").unwrap();
        assert_eq!(linked, Some(item_id));

        let unlinked = find_linked_item_id(&ctx, source.id, "guid-2").unwrap();
        assert_eq!(unlinked, None);
    }

    #[tokio::test]
    async fn a_malformed_query_fails_fast_without_touching_any_source() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = DiscoverService::discover(&ctx, "bogus:value", None, 25)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidQuery(_)));
    }
}
