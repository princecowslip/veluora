use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::LibraryRootId;

/// A local filesystem folder registered for scanning.
///
/// No entity for this exists in `docs/13-data-model.md` (it predates
/// scanning); this is the minimal reasonable shape for Milestone B,
/// following the style of the other entities in this crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRoot {
    pub id: LibraryRootId,
    pub path: String,
    pub display_name: Option<String>,
    pub enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_scanned_at: Option<OffsetDateTime>,
}

impl LibraryRoot {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            id: LibraryRootId::new(),
            path: path.into(),
            display_name: None,
            enabled: true,
            created_at: OffsetDateTime::now_utc(),
            last_scanned_at: None,
        }
    }
}
