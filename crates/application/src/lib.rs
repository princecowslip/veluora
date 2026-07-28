//! Application services: the seam ADR-002 requires between `local-api`,
//! `cli` (and, later, `gui`/`tui`) and the `domain`/`database` crates.
//! Presentation layers call through here — never around it.

pub mod collections;
pub mod comics;
pub mod context;
pub mod diagnostics;
pub mod error;
pub mod items;
pub mod library;
pub mod media_classification;
pub mod playback;
pub mod privacy;
pub mod scanner;
pub mod search;
pub mod settings;
pub mod stories;
pub mod thumbnails;
pub mod time_format;
pub mod user_state;

pub use collections::CollectionService;
pub use comics::{ComicService, PageSummary};
pub use context::AppContext;
pub use diagnostics::{DiagnosticsService, DiagnosticsSummary};
pub use error::{AppError, Result};
pub use items::{ItemDetail, ItemService, VariantSummary};
pub use library::{LibraryRootService, LibraryRootSummary, LibraryService, LibraryStatus};
pub use playback::{OpenTarget, PlaybackService, COMPLETION_THRESHOLD};
pub use privacy::PrivacyService;
pub use scanner::{RootScanResult, ScanReport, ScanService, SkippedFile};
pub use search::{SearchHit, SearchResults, SearchService};
pub use settings::{SettingsService, Theme};
pub use stories::StoryService;
pub use thumbnails::ThumbnailService;
pub use user_state::UserStateService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_summary_reports_all_applied_migrations() {
        let ctx = AppContext::open_in_memory().expect("context");
        let summary = DiagnosticsService::summary(&ctx).expect("summary");
        assert_eq!(
            summary.applied_migrations,
            database::migrations::MIGRATIONS.len() as i64
        );
        assert_eq!(summary.schema_version, 1);
    }

    #[test]
    fn library_status_starts_empty() {
        let ctx = AppContext::open_in_memory().expect("context");
        let status = LibraryService::status(&ctx).expect("status");
        assert_eq!(status.root_count, 0);
        assert_eq!(status.item_count, 0);
    }

    #[test]
    fn open_at_creates_data_dir_and_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("data");
        {
            let ctx = AppContext::open_at(&nested).expect("open");
            ctx.db
                .connection()
                .execute(
                    "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                     VALUES ('33333333-3333-3333-3333-333333333333', 'video', 'Persisted', 'unrated', datetime('now'), datetime('now'))",
                    [],
                )
                .unwrap();
        }
        let ctx = AppContext::open_at(&nested).expect("reopen");
        let status = LibraryService::status(&ctx).expect("status");
        assert_eq!(status.item_count, 1);
    }
}
