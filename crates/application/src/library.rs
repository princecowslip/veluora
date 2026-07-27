use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::Result;

/// Reports the local library's current state. Folder roots and scanning
/// don't exist yet (Milestone B, Workstream 4), so `root_count` is
/// permanently zero for now — `item_count` still reflects whatever has
/// been inserted directly, which is enough for Milestone A tests and
/// `veloura db check`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryStatus {
    pub root_count: u32,
    pub item_count: i64,
}

pub struct LibraryService;

impl LibraryService {
    pub fn status(ctx: &AppContext) -> Result<LibraryStatus> {
        let item_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |row| row.get(0))
            .map_err(database::DatabaseError::from)?;
        Ok(LibraryStatus {
            root_count: 0,
            item_count,
        })
    }
}
