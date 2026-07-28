//! SQLite-backed local storage.
//!
//! Owns the one thing the domain crate deliberately doesn't: I/O. See
//! `docs/13-data-model.md` for the schema this implements and ADR-003 in
//! `docs/26-architecture-decisions.md` for why SQLite.

pub mod error;
pub mod migrations;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

pub use error::{DatabaseError, Result};

/// An open connection to the local database, with WAL mode and foreign
/// keys enabled and all pending migrations applied.
///
/// The connection is wrapped in a `Mutex` so `Database` is `Sync` and can
/// sit behind an `Arc` in shared state (e.g. axum's router state in
/// `local-api`) — `rusqlite::Connection` itself is `Send` but not `Sync`.
pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    /// Open (creating if necessary) the database file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut conn = Connection::open(&path)?;
        Self::configure(&conn)?;
        migrations::run_migrations(&mut conn, &path)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// Open a private in-memory database. Used by tests and by any
    /// caller that explicitly wants a throwaway store.
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        let memory_path = PathBuf::from(":memory:");
        migrations::run_migrations(&mut conn, &memory_path)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: memory_path,
        })
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(())
    }

    pub fn connection(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .expect("database connection mutex poisoned")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of applied migrations — surfaced by `veloura db check` and
    /// `GET /diagnostics/summary`.
    pub fn applied_migration_count(&self) -> Result<i64> {
        Ok(self
            .connection()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_applies_migrations() {
        let db = Database::open_in_memory().expect("open");
        assert_eq!(
            db.applied_migration_count().unwrap(),
            migrations::MIGRATIONS.len() as i64
        );
    }

    #[test]
    fn schema_has_expected_tables() {
        let db = Database::open_in_memory().expect("open");
        let count: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'media_items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_0002_adds_library_roots_and_hashing_columns() {
        let db = Database::open_in_memory().expect("open");
        let library_roots_exists: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'library_roots'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(library_roots_exists, 1);

        db.connection()
            .execute(
                "INSERT INTO library_roots (id, path, enabled, created_at) VALUES ('r1', '/tmp/x', 1, datetime('now'))",
                [],
            )
            .unwrap();
        // Referencing the new media_variants columns fails at runtime if
        // migration 0002 didn't actually add them.
        db.connection()
            .execute(
                "UPDATE media_variants SET content_hash = 'abc', last_seen_at = datetime('now'), library_root_id = 'r1' WHERE 1 = 0",
                [],
            )
            .unwrap();
    }

    #[test]
    fn migration_0003_adds_story_documents_and_page_count() {
        let db = Database::open_in_memory().expect("open");
        let story_documents_exists: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'story_documents'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(story_documents_exists, 1);

        db.connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES ('44444444-4444-4444-4444-444444444444', 'story', 'A Story', 'unrated', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
        db.connection()
            .execute(
                "INSERT INTO story_documents (item_id, format, sanitized_content_location, chapter_map)
                 VALUES ('44444444-4444-4444-4444-444444444444', 'markdown', '/cache/stories/x.txt', '[]')",
                [],
            )
            .unwrap();

        // Referencing the new media_variants column fails at runtime if
        // migration 0003 didn't actually add it.
        db.connection()
            .execute("UPDATE media_variants SET page_count = 10 WHERE 1 = 0", [])
            .unwrap();
    }

    #[test]
    fn migration_0004_adds_app_settings() {
        let db = Database::open_in_memory().expect("open");
        db.connection()
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('theme', 'dark')",
                [],
            )
            .unwrap();
        let value: String = db
            .connection()
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "dark");
    }

    #[test]
    fn fts5_table_exists() {
        let db = Database::open_in_memory().expect("open");
        let count: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'media_items_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn inserts_and_reads_a_media_item_round_trip() {
        let db = Database::open_in_memory().expect("open");
        db.connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES ('11111111-1111-1111-1111-111111111111', 'video', 'Example', 'unrated', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
        let title: String = db
            .connection()
            .query_row(
                "SELECT title FROM media_items WHERE id = '11111111-1111-1111-1111-111111111111'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Example");
    }

    #[test]
    fn fts_stays_in_sync_with_inserted_titles() {
        let db = Database::open_in_memory().expect("open");
        db.connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES ('22222222-2222-2222-2222-222222222222', 'video', 'Searchable Title', 'unrated', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
        let matches: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM media_items_fts WHERE media_items_fts MATCH 'Searchable'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(matches, 1);
    }

    #[test]
    fn reopening_an_up_to_date_file_database_creates_no_backup() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("veloura.db");
        {
            // First open applies every migration, producing one backup per
            // migration applied.
            let _db = Database::open(&db_path).expect("open");
        }
        let backups_after_first_open = count_backups(dir.path());
        assert_eq!(backups_after_first_open, migrations::MIGRATIONS.len());

        // Second open: no pending migrations, so no *additional* backup.
        let _db = Database::open(&db_path).expect("reopen");
        assert_eq!(count_backups(dir.path()), backups_after_first_open);
    }

    fn count_backups(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .count()
    }
}
