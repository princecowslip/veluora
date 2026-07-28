use std::path::Path;

use domain::{LibraryRoot, LibraryRootId};
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::time_format::{from_rfc3339, to_rfc3339};

/// Reports the local library's current state: registered folder roots
/// and how many items have been indexed.
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryStatus {
    pub root_count: u32,
    pub item_count: i64,
    pub roots: Vec<LibraryRootSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryRootSummary {
    pub id: String,
    pub path: String,
    pub enabled: bool,
    pub last_scanned_at: Option<String>,
}

pub struct LibraryService;

impl LibraryService {
    pub fn status(ctx: &AppContext) -> Result<LibraryStatus> {
        let item_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |row| row.get(0))
            .map_err(database::DatabaseError::from)?;

        let roots = LibraryRootService::list(ctx)?;
        let root_count = roots.len() as u32;
        let summaries = roots
            .into_iter()
            .map(|root| LibraryRootSummary {
                id: root.id.to_string(),
                path: root.path,
                enabled: root.enabled,
                last_scanned_at: root.last_scanned_at.map(to_rfc3339),
            })
            .collect();

        Ok(LibraryStatus {
            root_count,
            item_count,
            roots: summaries,
        })
    }
}

pub struct LibraryRootService;

impl LibraryRootService {
    /// Canonicalizes `path`, rejects non-directories, and registers it.
    /// Duplicate paths are rejected by the `library_roots.path` UNIQUE
    /// constraint.
    pub fn add(ctx: &AppContext, path: &Path, display_name: Option<String>) -> Result<LibraryRoot> {
        let canonical = path
            .canonicalize()
            .map_err(|e| AppError::InvalidPath(format!("{}: {e}", path.display())))?;
        if !canonical.is_dir() {
            return Err(AppError::InvalidPath(format!(
                "{} is not a directory",
                canonical.display()
            )));
        }

        let mut root = LibraryRoot::new(canonical.display().to_string());
        root.display_name = display_name;

        ctx.db
            .connection()
            .execute(
                "INSERT INTO library_roots (id, path, display_name, enabled, created_at, last_scanned_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![
                    root.id.to_string(),
                    root.path,
                    root.display_name,
                    root.enabled as i64,
                    to_rfc3339(root.created_at),
                ],
            )
            .map_err(database::DatabaseError::from)?;

        Ok(root)
    }

    pub fn list(ctx: &AppContext) -> Result<Vec<LibraryRoot>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, path, display_name, enabled, created_at, last_scanned_at
                 FROM library_roots ORDER BY created_at",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map([], row_to_library_root)
            .map_err(database::DatabaseError::from)?;

        let mut roots = Vec::new();
        for row in rows {
            roots.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(roots)
    }

    pub fn find_by_path(ctx: &AppContext, path: &Path) -> Result<Option<LibraryRoot>> {
        let canonical = path
            .canonicalize()
            .map_err(|e| AppError::InvalidPath(format!("{}: {e}", path.display())))?;
        Self::query_one(
            ctx,
            "SELECT id, path, display_name, enabled, created_at, last_scanned_at
             FROM library_roots WHERE path = ?1",
            params![canonical.display().to_string()],
        )
    }

    pub fn find_by_id(ctx: &AppContext, id: LibraryRootId) -> Result<Option<LibraryRoot>> {
        Self::query_one(
            ctx,
            "SELECT id, path, display_name, enabled, created_at, last_scanned_at
             FROM library_roots WHERE id = ?1",
            params![id.to_string()],
        )
    }

    fn query_one(
        ctx: &AppContext,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Option<LibraryRoot>> {
        let conn = ctx.db.connection();
        match conn.query_row(sql, params, row_to_library_root) {
            Ok(root) => Ok(Some(root)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(database::DatabaseError::from(e).into()),
        }
    }

    /// Detaches (never deletes) the `MediaVariant` rows scoped to this
    /// root, then deletes the root row itself. `MediaItem`, `UserState`
    /// (favorites/ratings/notes), tags, and collection membership are
    /// always preserved — content hashes are retained on the detached
    /// variants so a later scan elsewhere can still relink them.
    pub fn remove(ctx: &AppContext, id: LibraryRootId) -> Result<()> {
        let conn = ctx.db.connection();
        conn.execute(
            "UPDATE media_variants SET local_path = NULL, library_root_id = NULL WHERE library_root_id = ?1",
            params![id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;

        let affected = conn
            .execute(
                "DELETE FROM library_roots WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("library root {id}")));
        }
        Ok(())
    }
}

fn row_to_library_root(row: &Row) -> rusqlite::Result<LibraryRoot> {
    let id_str: String = row.get(0)?;
    let path: String = row.get(1)?;
    let display_name: Option<String> = row.get(2)?;
    let enabled: bool = row.get(3)?;
    let created_at_str: String = row.get(4)?;
    let last_scanned_at_str: Option<String> = row.get(5)?;

    let id = LibraryRootId(Uuid::parse_str(&id_str).unwrap_or_default());
    let created_at = from_rfc3339(&created_at_str).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let last_scanned_at = last_scanned_at_str.as_deref().and_then(from_rfc3339);

    Ok(LibraryRoot {
        id,
        path,
        display_name,
        enabled,
        created_at,
        last_scanned_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AppContext;

    #[test]
    fn add_rejects_a_path_that_is_not_a_directory() {
        let ctx = AppContext::open_in_memory().expect("context");
        let file = tempfile::NamedTempFile::new().unwrap();
        let err = LibraryRootService::add(&ctx, file.path(), None).unwrap_err();
        assert!(matches!(err, AppError::InvalidPath(_)));
    }

    #[test]
    fn add_list_and_find_round_trip() {
        let ctx = AppContext::open_in_memory().expect("context");
        let dir = tempfile::tempdir().unwrap();

        let added =
            LibraryRootService::add(&ctx, dir.path(), Some("My Library".to_string())).unwrap();
        assert_eq!(added.display_name.as_deref(), Some("My Library"));

        let listed = LibraryRootService::list(&ctx).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, added.id);

        let found = LibraryRootService::find_by_path(&ctx, dir.path()).unwrap();
        assert_eq!(found.unwrap().id, added.id);

        let missing =
            LibraryRootService::find_by_path(&ctx, std::path::Path::new("/nonexistent-xyz"))
                .unwrap_err();
        assert!(matches!(missing, AppError::InvalidPath(_)));
    }

    #[test]
    fn adding_the_same_path_twice_fails() {
        let ctx = AppContext::open_in_memory().expect("context");
        let dir = tempfile::tempdir().unwrap();

        LibraryRootService::add(&ctx, dir.path(), None).unwrap();
        let err = LibraryRootService::add(&ctx, dir.path(), None).unwrap_err();
        assert!(matches!(err, AppError::Database(_)));
    }

    #[test]
    fn remove_detaches_variants_and_preserves_items() {
        let ctx = AppContext::open_in_memory().expect("context");
        let dir = tempfile::tempdir().unwrap();
        let root = LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let conn = ctx.db.connection();
        conn.execute(
            "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
             VALUES ('44444444-4444-4444-4444-444444444444', 'image', 'Pic', 'unrated', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, library_root_id, download_permitted, cache_permitted)
             VALUES ('55555555-5555-5555-5555-555555555555', '44444444-4444-4444-4444-444444444444', 'image/png', 'png', '/x/pic.png', ?1, 1, 1)",
            params![root.id.to_string()],
        )
        .unwrap();
        drop(conn);

        LibraryRootService::remove(&ctx, root.id).unwrap();

        let conn = ctx.db.connection();
        let item_still_present: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE id = '44444444-4444-4444-4444-444444444444'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(item_still_present, 1, "item must survive root removal");

        let (local_path, library_root_id): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT local_path, library_root_id FROM media_variants WHERE id = '55555555-5555-5555-5555-555555555555'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(local_path, None, "detached variant must clear local_path");
        assert_eq!(
            library_root_id, None,
            "detached variant must clear library_root_id"
        );

        let roots_remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM library_roots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(roots_remaining, 0);
    }

    #[test]
    fn removing_an_unknown_root_is_not_found() {
        let ctx = AppContext::open_in_memory().expect("context");
        let err = LibraryRootService::remove(&ctx, LibraryRootId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
