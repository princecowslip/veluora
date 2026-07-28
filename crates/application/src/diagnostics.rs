use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::Result;
use crate::library::LibraryRootService;
use crate::privacy::PrivacyService;
use crate::time_format::to_rfc3339;

/// Backs both `GET /diagnostics/summary` and `veloura doctor`, per
/// `docs/19-local-api.md` and `docs/10-cli.md`.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsSummary {
    pub schema_version: u32,
    pub applied_migrations: i64,
    pub data_dir: String,
    pub db_path: String,
    /// Whether `ffprobe` is reachable on `PATH` — video/audio probing
    /// (duration, dimensions, bitrate) is silently skipped when it
    /// isn't. See `docs/45-required-packages-dependencies.md`.
    pub ffprobe_available: bool,
    /// Whether `ffmpeg` is reachable on `PATH` — video thumbnail
    /// generation is silently skipped when it isn't.
    pub ffmpeg_available: bool,
}

/// A redacted, aggregate-only diagnostic snapshot for sharing with
/// support — never titles, paths, tags, notes, or thumbnails, per
/// `docs/23-operations-and-observability.md`'s support-bundle rules.
#[derive(Debug, Serialize, Deserialize)]
pub struct SupportBundle {
    pub schema_version: u32,
    pub app_version: String,
    pub os: String,
    /// The highest schema version this build knows how to apply —
    /// distinct from `applied_migrations`, which is how many have
    /// actually run against this particular database.
    pub db_migration_version: i64,
    pub applied_migrations: i64,
    pub library_root_count: u32,
    pub item_counts_by_media_type: BTreeMap<String, i64>,
    pub cache_size_bytes: u64,
    pub ffprobe_available: bool,
    pub ffmpeg_available: bool,
    pub metadata_encryption_enabled: bool,
    pub generated_at: String,
}

pub struct DiagnosticsService;

impl DiagnosticsService {
    pub fn summary(ctx: &AppContext) -> Result<DiagnosticsSummary> {
        Ok(DiagnosticsSummary {
            schema_version: 1,
            applied_migrations: ctx.db.applied_migration_count()?,
            data_dir: ctx.data_dir.display().to_string(),
            db_path: ctx.db.path().display().to_string(),
            ffprobe_available: media::ffprobe_available(),
            ffmpeg_available: media::ffmpeg_available(),
        })
    }

    pub fn support_bundle(ctx: &AppContext) -> Result<SupportBundle> {
        let library_root_count = LibraryRootService::list(ctx)?.len() as u32;

        let item_counts_by_media_type = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare("SELECT media_type, COUNT(*) FROM media_items GROUP BY media_type")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(database::DatabaseError::from)?;
            let mut map = BTreeMap::new();
            for row in rows {
                let (media_type, count) = row.map_err(database::DatabaseError::from)?;
                map.insert(media_type, count);
            }
            map
        };

        Ok(SupportBundle {
            schema_version: 1,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            db_migration_version: database::migrations::MIGRATIONS.len() as i64,
            applied_migrations: ctx.db.applied_migration_count()?,
            library_root_count,
            item_counts_by_media_type,
            cache_size_bytes: PrivacyService::cache_size_bytes(ctx)?,
            ffprobe_available: media::ffprobe_available(),
            ffmpeg_available: media::ffmpeg_available(),
            metadata_encryption_enabled: PrivacyService::metadata_encryption_enabled(ctx)?,
            generated_at: to_rfc3339(time::OffsetDateTime::now_utc()),
        })
    }

    /// Hot-backs-up the live database to `destination`.
    pub fn export_backup(ctx: &AppContext, destination: &Path) -> Result<()> {
        ctx.db.backup_to(destination)?;
        Ok(())
    }

    /// Validates and restores from `source`, replacing the live
    /// database file. **Restart the process** afterward — there's no
    /// in-process hot-swap of the connection this milestone. See
    /// `database::Database::restore_from` for the validation rules.
    pub fn restore_backup(ctx: &AppContext, source: &Path) -> Result<()> {
        database::Database::restore_from(source, ctx.db.path())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AppContext;

    #[test]
    fn support_bundle_counts_items_by_media_type_and_library_roots() {
        let ctx = AppContext::open_in_memory().unwrap();
        for (id, media_type, title) in [
            (
                "11111111-1111-1111-1111-111111111111",
                "video",
                "A Secret Home Video",
            ),
            (
                "22222222-2222-2222-2222-222222222222",
                "video",
                "Another Video",
            ),
            (
                "33333333-3333-3333-3333-333333333333",
                "image",
                "A Private Photo",
            ),
        ] {
            ctx.db
                .connection()
                .execute(
                    "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                     VALUES (?1, ?2, ?3, 'unrated', datetime('now'), datetime('now'))",
                    rusqlite::params![id, media_type, title],
                )
                .unwrap();
        }

        let bundle = DiagnosticsService::support_bundle(&ctx).unwrap();
        assert_eq!(bundle.item_counts_by_media_type.get("video"), Some(&2));
        assert_eq!(bundle.item_counts_by_media_type.get("image"), Some(&1));
        assert_eq!(bundle.library_root_count, 0);
        assert!(!bundle.metadata_encryption_enabled);
    }

    #[test]
    fn support_bundle_never_contains_titles_or_local_paths() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        let secret_title = "My Extremely Private Vacation Video";
        let secret_path = "/home/someone/very/private/folder/clip.mp4";
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES ('44444444-4444-4444-4444-444444444444', 'video', ?1, 'unrated', datetime('now'), datetime('now'))",
                rusqlite::params![secret_title],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, download_permitted, cache_permitted)
                 VALUES ('55555555-5555-5555-5555-555555555555', '44444444-4444-4444-4444-444444444444', 'video/mp4', 'mp4', ?1, 1, 1)",
                rusqlite::params![secret_path],
            )
            .unwrap();

        let bundle = DiagnosticsService::support_bundle(&ctx).unwrap();
        let serialized = serde_json::to_string(&bundle).unwrap();
        assert!(!serialized.contains(secret_title));
        assert!(!serialized.contains(secret_path));
        assert!(!serialized.contains("/home/someone"));
    }

    #[test]
    fn export_backup_writes_a_file_that_is_independently_restorable() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES ('66666666-6666-6666-6666-666666666666', 'video', 'Round Trip', 'unrated', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();

        let backup_path = dir.path().join("export.db");
        DiagnosticsService::export_backup(&ctx, &backup_path).unwrap();
        assert!(backup_path.exists());

        // Restoring safely requires no live connection to db_path be open
        // (the real restart requirement — see restore_backup's doc
        // comment), so drop `ctx` first; the round-trip mechanics
        // themselves are already covered by
        // database::tests::backup_to_and_restore_from_round_trips_the_data.
        let db_path = ctx.db.path().to_path_buf();
        drop(ctx);
        database::Database::restore_from(&backup_path, &db_path).unwrap();

        let reopened = AppContext::open_at(dir.path()).unwrap();
        let title: String = reopened
            .db
            .connection()
            .query_row(
                "SELECT title FROM media_items WHERE id = '66666666-6666-6666-6666-666666666666'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Round Trip");
    }

    #[test]
    fn restore_backup_rejects_an_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        let bogus = dir.path().join("bogus.db");
        std::fs::write(&bogus, b"not a database").unwrap();

        let err = DiagnosticsService::restore_backup(&ctx, &bogus).unwrap_err();
        assert!(matches!(err, crate::error::AppError::Database(_)));
    }
}
