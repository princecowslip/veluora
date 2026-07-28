//! External-tool and format-specific media handling: `ffprobe`/`ffmpeg`
//! subprocess probing, CBZ archive reading, plain-text/Markdown story
//! ingestion, and external player launching.
//!
//! Kept separate from `application` so the orchestration/persistence
//! layer doesn't have to know how any of this works — mirrors how
//! `scanner.rs` already isolates filesystem-walking concerns. Nothing
//! here touches the database; callers in `application` decide what to
//! persist.

pub mod archive;
pub mod error;
pub mod external_player;
pub mod frame;
pub mod probe;
pub mod story;

pub use archive::{list_pages, read_page, ArchivePage};
pub use error::{MediaError, Result};
pub use external_player::{build_command, launch, PlayerCommand};
pub use frame::extract_frame_png;
pub use probe::{ffmpeg_available, ffprobe_available, probe, MediaProbe};
pub use story::{build_story_document, StoryContent};
