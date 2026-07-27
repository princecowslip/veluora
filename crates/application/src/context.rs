use std::path::{Path, PathBuf};

use database::Database;
use directories::ProjectDirs;

use crate::error::{AppError, Result};

/// Shared state constructed once and handed to every application
/// service. Both `local-api` and `cli` go through this — never through
/// `database`/`domain` directly — per ADR-002 in
/// `docs/26-architecture-decisions.md`.
pub struct AppContext {
    pub db: Database,
    pub data_dir: PathBuf,
}

impl AppContext {
    /// Resolve the OS-appropriate data directory, ensure it exists, and
    /// open (creating if necessary) `veloura.db` inside it.
    pub fn open_default() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "veloura").ok_or(AppError::NoDataDirectory)?;
        let data_dir = dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        Self::open_at(&data_dir)
    }

    /// Open the database inside a specific data directory. Exposed for the
    /// CLI's `--config`/data-dir overrides and for tests.
    pub fn open_at(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("veloura.db");
        let db = Database::open(db_path)?;
        Ok(Self {
            db,
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// An ephemeral, in-memory context for tests.
    pub fn open_in_memory() -> Result<Self> {
        let db = Database::open_in_memory()?;
        Ok(Self {
            db,
            data_dir: PathBuf::from(":memory:"),
        })
    }
}
