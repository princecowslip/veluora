//! Application services: the seam ADR-002 requires between `local-api`,
//! `cli` (and, later, `gui`/`tui`) and the `domain`/`database` crates.
//! Presentation layers call through here — never around it.

pub mod context;
pub mod diagnostics;
pub mod error;
pub mod library;

pub use context::AppContext;
pub use diagnostics::{DiagnosticsService, DiagnosticsSummary};
pub use error::{AppError, Result};
pub use library::{LibraryService, LibraryStatus};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_summary_reports_one_applied_migration() {
        let ctx = AppContext::open_in_memory().expect("context");
        let summary = DiagnosticsService::summary(&ctx).expect("summary");
        assert_eq!(summary.applied_migrations, 1);
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
