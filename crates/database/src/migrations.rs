//! Versioned SQL migrations, embedded at compile time from `/migrations`.
//!
//! Each migration runs inside its own transaction and the tracking table
//! records which versions have applied. The database file is copied aside
//! before a migration runs (when it's a real file, not `:memory:`), giving
//! a tested restore path per `docs/13-data-model.md`'s database guidance
//! and Workstream 3's acceptance criteria in `docs/46-implementation-plan.md`.

use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::{DatabaseError, Result};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../../../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "library_and_search",
        sql: include_str!("../../../migrations/0002_library_and_search.sql"),
    },
    Migration {
        version: 3,
        name: "media_documents",
        sql: include_str!("../../../migrations/0003_media_documents.sql"),
    },
    Migration {
        version: 4,
        name: "app_settings",
        sql: include_str!("../../../migrations/0004_app_settings.sql"),
    },
    Migration {
        version: 5,
        name: "pinned",
        sql: include_str!("../../../migrations/0005_pinned.sql"),
    },
    Migration {
        version: 6,
        name: "downloads",
        sql: include_str!("../../../migrations/0006_downloads.sql"),
    },
];

pub fn run_migrations(conn: &mut Connection, db_path: &Path) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            params![migration.version],
            |row| row.get(0),
        )?;
        if already_applied {
            continue;
        }

        backup_before_migration(db_path, migration.version)?;

        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)
            .map_err(|e| DatabaseError::Migration {
                version: migration.version,
                message: e.to_string(),
            })?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
            params![migration.version, migration.name],
        )?;
        tx.commit()?;
    }

    Ok(())
}

fn backup_before_migration(db_path: &Path, version: i64) -> Result<()> {
    if db_path.as_os_str() == ":memory:" || !db_path.exists() {
        // Nothing to protect: in-memory database. A real file is backed
        // up even on the very first migration — `Connection::open`
        // already wrote a SQLite header to it via `configure()` by the
        // time this runs, so it's never truly empty on disk.
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_path = db_path.with_extension(format!("db.bak-{version}-{stamp}"));
    fs::copy(db_path, backup_path)?;
    Ok(())
}
