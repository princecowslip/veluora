//! SQLite-backed local storage.
//!
//! Owns the one thing the domain crate deliberately doesn't: I/O. See
//! `docs/13-data-model.md` for the schema this implements and ADR-003 in
//! `docs/26-architecture-decisions.md` for why SQLite.

pub mod error;
pub mod migrations;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

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

    /// Hot-backs-up the live database to `destination` using SQLite's
    /// native online-backup API — safe to run against a WAL-mode
    /// database that's actively being read/written, unlike a raw file
    /// copy.
    pub fn backup_to(&self, destination: &Path) -> Result<()> {
        let mut dest_conn = Connection::open(destination)?;
        let source = self.connection();
        let backup = rusqlite::backup::Backup::new(&source, &mut dest_conn)?;
        backup.run_to_completion(100, Duration::from_millis(50), None)?;
        Ok(())
    }

    /// Validates `source` as a restorable backup (opens cleanly, passes
    /// `PRAGMA integrity_check`, and doesn't claim a schema version
    /// newer than this build understands), backs up the live database
    /// at `db_path` first as a safety net, then copies `source` over it
    /// — including removing stale `-wal`/`-shm` sidecar files left by
    /// WAL mode. There's no in-process hot-swap of the live connection
    /// this milestone: the caller must restart the process for the
    /// restored data to take effect.
    pub fn restore_from(source: &Path, db_path: &Path) -> Result<()> {
        {
            // Opened read-write (not the live DB — this is the candidate
            // backup file): FTS5's integrity check needs to validate its
            // shadow tables, which SQLite refuses to do against a
            // read-only connection.
            let source_conn = Connection::open(source)?;

            let integrity: String =
                source_conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
            if integrity != "ok" {
                return Err(DatabaseError::Backup(format!(
                    "backup failed integrity check: {integrity}"
                )));
            }

            let max_version: Option<i64> = source_conn
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .map_err(|_| {
                    DatabaseError::Backup(
                        "backup is not a valid Veloura database (missing schema_migrations table)"
                            .to_string(),
                    )
                })?;
            let max_version = max_version.unwrap_or(0);
            if max_version > migrations::MIGRATIONS.len() as i64 {
                return Err(DatabaseError::Backup(format!(
                    "backup schema version {max_version} is newer than this build understands ({} known)",
                    migrations::MIGRATIONS.len()
                )));
            }
        }

        if db_path.exists() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let safety_backup = db_path.with_extension(format!("db.pre-restore-{stamp}"));
            std::fs::copy(db_path, &safety_backup)?;
        }

        std::fs::copy(source, db_path)?;

        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", db_path.display()));
            if sidecar.exists() {
                std::fs::remove_file(sidecar)?;
            }
        }

        Ok(())
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

    #[test]
    fn backup_to_and_restore_from_round_trips_the_data() {
        let dir = tempfile::tempdir().unwrap();
        let live_path = dir.path().join("live.db");
        let backup_path = dir.path().join("export.db");

        {
            let db = Database::open(&live_path).expect("open live");
            db.connection()
                .execute(
                    "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                     VALUES ('11111111-1111-1111-1111-111111111111', 'video', 'Backed Up', 'unrated', datetime('now'), datetime('now'))",
                    [],
                )
                .unwrap();
            db.backup_to(&backup_path).expect("backup");
        }

        // Simulate the live DB changing (or being lost) after the backup.
        {
            let db = Database::open(&live_path).expect("reopen live");
            db.connection()
                .execute("DELETE FROM media_items", [])
                .unwrap();
        }

        Database::restore_from(&backup_path, &live_path).expect("restore");

        let db = Database::open(&live_path).expect("reopen restored");
        let title: String = db
            .connection()
            .query_row(
                "SELECT title FROM media_items WHERE id = '11111111-1111-1111-1111-111111111111'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Backed Up");
    }

    #[test]
    fn restore_from_rejects_a_backup_with_a_newer_schema_version_than_this_build_knows() {
        let dir = tempfile::tempdir().unwrap();
        let live_path = dir.path().join("live.db");
        let future_backup_path = dir.path().join("future.db");

        let _db = Database::open(&live_path).expect("open live");

        {
            let conn = Connection::open(&future_backup_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL);
                 INSERT INTO schema_migrations (version, name, applied_at) VALUES (999, 'from_the_future', datetime('now'));",
            )
            .unwrap();
        }

        let err = Database::restore_from(&future_backup_path, &live_path).unwrap_err();
        assert!(matches!(err, DatabaseError::Backup(_)));
    }

    #[test]
    fn restore_from_rejects_a_file_that_is_not_a_veloura_database() {
        let dir = tempfile::tempdir().unwrap();
        let live_path = dir.path().join("live.db");
        let bogus_path = dir.path().join("bogus.db");

        let _db = Database::open(&live_path).expect("open live");
        {
            let conn = Connection::open(&bogus_path).unwrap();
            conn.execute_batch("CREATE TABLE unrelated (id INTEGER PRIMARY KEY);")
                .unwrap();
        }

        let err = Database::restore_from(&bogus_path, &live_path).unwrap_err();
        assert!(matches!(err, DatabaseError::Backup(_)));
    }
}
