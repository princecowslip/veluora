use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::ItemId;

/// Format-specific playback/reading position.
///
/// A normalized percentage can be derived for display, but the native
/// position is retained for accuracy (per `docs/13-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "progress_type", rename_all = "snake_case")]
pub enum Progress {
    TimeBased {
        position_ms: u64,
        duration_ms: Option<u64>,
    },
    Story {
        character_offset: u64,
        chapter_index: u32,
    },
    Comic {
        page_index: u32,
        intra_page_position: f32,
    },
    Gallery {
        item_index: u32,
    },
    Image {
        viewed: bool,
    },
}

impl Progress {
    /// Normalized 0.0–1.0 completion, where it can be computed.
    pub fn normalized(&self) -> Option<f32> {
        match self {
            Progress::TimeBased {
                position_ms,
                duration_ms: Some(d),
            } if *d > 0 => Some((*position_ms as f32 / *d as f32).clamp(0.0, 1.0)),
            Progress::Image { viewed } => Some(if *viewed { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
}

/// A user's local relationship to an item: favorites, personal rating,
/// viewing/queue state, notes, and private tags. Distinct from
/// [`crate::media_item::MediaItem::rating_classification`], which is
/// source/content classification rather than the user's own rating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserState {
    pub item_id: ItemId,
    pub favorite: bool,
    pub rating: Option<u8>,
    pub viewed: bool,
    pub completed: bool,
    pub progress: Option<Progress>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_opened_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub queued_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    pub private_tags: Vec<String>,
}

impl UserState {
    pub fn new(item_id: ItemId) -> Self {
        Self {
            item_id,
            favorite: false,
            rating: None,
            viewed: false,
            completed: false,
            progress: None,
            last_opened_at: None,
            queued_at: None,
            notes: None,
            private_tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_based_progress_normalizes() {
        let p = Progress::TimeBased {
            position_ms: 5_000,
            duration_ms: Some(10_000),
        };
        assert_eq!(p.normalized(), Some(0.5));
    }

    #[test]
    fn story_progress_has_no_normalized_value_yet() {
        let p = Progress::Story {
            character_offset: 120,
            chapter_index: 2,
        };
        assert_eq!(p.normalized(), None);
    }
}
