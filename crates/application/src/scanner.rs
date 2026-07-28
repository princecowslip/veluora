//! Folder scanning: classify, hash, and index files under a
//! [`domain::LibraryRoot`].
//!
//! "Resume safely" (Workstream 4's acceptance criteria in
//! `docs/46-implementation-plan.md`) is implemented as idempotent
//! re-scanning rather than a checkpoint table: matching is
//! content-addressed (path, then BLAKE3 hash), so restarting an
//! interrupted scan from the beginning converges to the same end state a
//! completed scan would reach. Each file's item+variant insert commits
//! in its own transaction, so a crash loses at most the in-flight file.

use std::path::Path;

use domain::{ItemId, LibraryRoot, LibraryRootId, MediaType, StoryFormat, VariantId};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::library::LibraryRootService;
use crate::media_classification::{classify, is_ignored_filename, media_type_to_str};
use crate::stories::StoryService;
use crate::thumbnails::ThumbnailService;
use crate::time_format::to_rfc3339;

const MAX_SKIPPED_LISTED: usize = 200;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScanReport {
    pub schema_version: u32,
    pub roots: Vec<RootScanResult>,
    pub skipped: Vec<SkippedFile>,
    pub skipped_total: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RootScanResult {
    pub root_id: String,
    pub path: String,
    pub files_seen: u32,
    pub added: u32,
    pub updated: u32,
    pub moved: u32,
    pub missing: u32,
    pub unsupported: u32,
    /// Set only if the root itself couldn't be walked (e.g. the folder
    /// vanished) — other roots in the same `scan_all` call still proceed.
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

pub struct ScanService;

impl ScanService {
    /// Scans every enabled root. A single root's failure is captured in
    /// its `RootScanResult.error` and does not stop the others.
    pub fn scan_all(ctx: &AppContext) -> Result<ScanReport> {
        let roots: Vec<LibraryRoot> = LibraryRootService::list(ctx)?
            .into_iter()
            .filter(|r| r.enabled)
            .collect();
        let mut report = ScanReport {
            schema_version: 1,
            ..Default::default()
        };
        for root in &roots {
            let result = scan_one_root(ctx, root, &mut report);
            report.roots.push(result);
        }
        Ok(report)
    }

    pub fn scan_root(ctx: &AppContext, root_id: LibraryRootId) -> Result<ScanReport> {
        let root = LibraryRootService::find_by_id(ctx, root_id)?
            .ok_or_else(|| AppError::NotFound(format!("library root {root_id}")))?;
        let mut report = ScanReport {
            schema_version: 1,
            ..Default::default()
        };
        let result = scan_one_root(ctx, &root, &mut report);
        report.roots.push(result);
        Ok(report)
    }

    /// `path` must already be a registered root (added via
    /// [`LibraryRootService::add`]) — this does not silently register
    /// new roots, matching `docs/10-cli.md`'s example ordering of
    /// `library add` before `library scan --path`.
    pub fn scan_path(ctx: &AppContext, path: &Path) -> Result<ScanReport> {
        let root = LibraryRootService::find_by_path(ctx, path)?.ok_or_else(|| {
            AppError::NotFound(format!(
                "{} is not a registered library root — run `library add` first",
                path.display()
            ))
        })?;
        let mut report = ScanReport {
            schema_version: 1,
            ..Default::default()
        };
        let result = scan_one_root(ctx, &root, &mut report);
        report.roots.push(result);
        Ok(report)
    }
}

enum FileOutcome {
    Added,
    Updated,
    Moved,
    Unchanged,
}

struct ExistingVariant {
    id: VariantId,
    item_id: ItemId,
    local_path: Option<String>,
    file_size: Option<u64>,
    mtime_unix: Option<i64>,
}

fn scan_one_root(ctx: &AppContext, root: &LibraryRoot, report: &mut ScanReport) -> RootScanResult {
    let mut result = RootScanResult {
        root_id: root.id.to_string(),
        path: root.path.clone(),
        ..Default::default()
    };

    let root_path = Path::new(&root.path);
    if !root_path.is_dir() {
        result.error = Some(format!("{} is not accessible", root.path));
        return result;
    }

    let scan_started_at = to_rfc3339(OffsetDateTime::now_utc());

    for entry in walkdir::WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if is_ignored_filename(path) {
            continue;
        }
        result.files_seen += 1;

        let Some((media_type, format)) = classify(path) else {
            record_skip(report, path, "unsupported extension");
            result.unsupported += 1;
            continue;
        };

        let now = to_rfc3339(OffsetDateTime::now_utc());
        match process_file(ctx, root, path, media_type, format, &now) {
            Ok(FileOutcome::Added) => result.added += 1,
            Ok(FileOutcome::Updated) => result.updated += 1,
            Ok(FileOutcome::Moved) => result.moved += 1,
            Ok(FileOutcome::Unchanged) => {}
            Err(e) => {
                record_skip(report, path, &e.to_string());
                result.unsupported += 1;
            }
        }
    }

    match mark_missing(ctx, root.id, &scan_started_at) {
        Ok(count) => result.missing = count,
        Err(e) => result.error = Some(format!("missing-file detection failed: {e}")),
    }

    let now = to_rfc3339(OffsetDateTime::now_utc());
    if let Err(e) = update_root_last_scanned(ctx, root.id, &now) {
        result.error = Some(e.to_string());
    }

    result
}

fn record_skip(report: &mut ScanReport, path: &Path, reason: &str) {
    report.skipped_total += 1;
    if report.skipped.len() < MAX_SKIPPED_LISTED {
        report.skipped.push(SkippedFile {
            path: path.display().to_string(),
            reason: reason.to_string(),
        });
    }
}

fn process_file(
    ctx: &AppContext,
    root: &LibraryRoot,
    path: &Path,
    media_type: MediaType,
    format: &str,
    now: &str,
) -> Result<FileOutcome> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    let mtime = mtime_unix(&metadata);
    let path_str = path.display().to_string();

    let existing_at_path = find_variant_by_path(ctx, &path_str)?;

    if let Some(existing) = &existing_at_path {
        if existing.file_size == Some(file_size) && existing.mtime_unix == mtime {
            touch_last_seen(ctx, existing.id, now)?;
            return Ok(FileOutcome::Unchanged);
        }
    }

    let content_hash = hash_file(path)?;
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    if let Some(existing) = existing_at_path {
        update_variant_content(
            ctx,
            existing.id,
            &content_hash,
            file_size,
            mtime,
            &mime_type,
            format,
            now,
        )?;
        maybe_probe_and_thumbnail(ctx, media_type, existing.item_id, existing.id, format, path);
        return Ok(FileOutcome::Updated);
    }

    if let Some(existing) = find_variant_by_hash(ctx, &content_hash)? {
        let old_path_still_exists = existing
            .local_path
            .as_deref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false);
        if !old_path_still_exists {
            relink_variant(ctx, existing.id, root.id, &path_str, file_size, mtime, now)?;
            maybe_probe_and_thumbnail(ctx, media_type, existing.item_id, existing.id, format, path);
            return Ok(FileOutcome::Moved);
        }
    }

    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let (item_id, variant_id) = insert_new_item_and_variant(
        ctx,
        &title,
        media_type,
        format,
        root,
        &path_str,
        &mime_type,
        file_size,
        mtime,
        &content_hash,
        now,
    )?;
    maybe_probe_and_thumbnail(ctx, media_type, item_id, variant_id, format, path);
    Ok(FileOutcome::Added)
}

/// Per-type probing and thumbnail generation, run after every add/update/
/// move. Every branch is best-effort: probing tools that aren't
/// installed, corrupt files, or archives that fail safety checks never
/// fail the scan — they just leave that variant's metadata unpopulated,
/// same as always been true for images here.
fn maybe_probe_and_thumbnail(
    ctx: &AppContext,
    media_type: MediaType,
    item_id: ItemId,
    variant_id: VariantId,
    format: &str,
    path: &Path,
) {
    match media_type {
        MediaType::Image => {
            if let Ok((width, height)) = image::image_dimensions(path) {
                let _ = update_variant_dimensions(ctx, variant_id, width, height);
            }
            let _ = ThumbnailService::ensure(ctx, variant_id, path);
        }
        MediaType::Video => {
            if let Some(probe) = media::probe(path) {
                let _ = update_variant_probe(
                    ctx,
                    variant_id,
                    probe.width,
                    probe.height,
                    probe.duration_ms,
                    probe.bitrate,
                );
            }
            let _ = ThumbnailService::ensure_video_frame(ctx, variant_id, path);
        }
        MediaType::Audio => {
            if let Some(probe) = media::probe(path) {
                let _ = update_variant_probe(
                    ctx,
                    variant_id,
                    None,
                    None,
                    probe.duration_ms,
                    probe.bitrate,
                );
            }
        }
        MediaType::Comic | MediaType::Manga => {
            if let Ok(pages) = media::list_pages(path) {
                let _ = update_variant_page_count(ctx, variant_id, pages.len() as u32);
            }
            let _ = ThumbnailService::ensure_comic_page(ctx, variant_id, path);
        }
        MediaType::Story => {
            // EPUB stays classified but unread this milestone (out of
            // scope) — only plain text and Markdown get ingested.
            if let Some(story_format) = story_format_for(format) {
                let _ = StoryService::ensure(ctx, item_id, story_format, path);
            }
        }
        MediaType::Gallery | MediaType::Other => {}
    }
}

fn story_format_for(format: &str) -> Option<StoryFormat> {
    match format {
        "md" => Some(StoryFormat::Markdown),
        "txt" => Some(StoryFormat::PlainText),
        _ => None,
    }
}

fn mtime_unix(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

fn hash_file(path: &Path) -> std::result::Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn find_variant_by_path(ctx: &AppContext, path_str: &str) -> Result<Option<ExistingVariant>> {
    query_existing_variant(
        ctx,
        "SELECT id, item_id, local_path, file_size, mtime_unix FROM media_variants WHERE local_path = ?1 LIMIT 1",
        params![path_str],
    )
}

fn find_variant_by_hash(ctx: &AppContext, hash: &str) -> Result<Option<ExistingVariant>> {
    query_existing_variant(
        ctx,
        "SELECT id, item_id, local_path, file_size, mtime_unix FROM media_variants WHERE content_hash = ?1 LIMIT 1",
        params![hash],
    )
}

fn query_existing_variant(
    ctx: &AppContext,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Option<ExistingVariant>> {
    let conn = ctx.db.connection();
    let result = conn.query_row(sql, params, |row| {
        let id_str: String = row.get(0)?;
        let item_id_str: String = row.get(1)?;
        Ok(ExistingVariant {
            id: VariantId(Uuid::parse_str(&id_str).unwrap_or_default()),
            item_id: ItemId(Uuid::parse_str(&item_id_str).unwrap_or_default()),
            local_path: row.get(2)?,
            file_size: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
            mtime_unix: row.get(4)?,
        })
    });
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(database::DatabaseError::from(e).into()),
    }
}

fn touch_last_seen(ctx: &AppContext, variant_id: VariantId, now: &str) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants SET last_seen_at = ?1 WHERE id = ?2",
            params![now, variant_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_variant_content(
    ctx: &AppContext,
    variant_id: VariantId,
    content_hash: &str,
    file_size: u64,
    mtime: Option<i64>,
    mime_type: &str,
    format: &str,
    now: &str,
) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants
             SET content_hash = ?1, file_size = ?2, mtime_unix = ?3, mime_type = ?4, format = ?5, last_seen_at = ?6
             WHERE id = ?7",
            params![content_hash, file_size as i64, mtime, mime_type, format, now, variant_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

fn relink_variant(
    ctx: &AppContext,
    variant_id: VariantId,
    root_id: LibraryRootId,
    new_path: &str,
    file_size: u64,
    mtime: Option<i64>,
    now: &str,
) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants
             SET local_path = ?1, library_root_id = ?2, file_size = ?3, mtime_unix = ?4, last_seen_at = ?5
             WHERE id = ?6",
            params![new_path, root_id.to_string(), file_size as i64, mtime, now, variant_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

fn update_variant_dimensions(
    ctx: &AppContext,
    variant_id: VariantId,
    width: u32,
    height: u32,
) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants SET width = ?1, height = ?2 WHERE id = ?3",
            params![width, height, variant_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

/// Applies an `ffprobe` result to a variant. Each field is only
/// overwritten when the probe actually produced a value (`COALESCE`),
/// so an audio probe (no width/height) doesn't clobber a value another
/// pass might have set.
fn update_variant_probe(
    ctx: &AppContext,
    variant_id: VariantId,
    width: Option<u32>,
    height: Option<u32>,
    duration_ms: Option<u64>,
    bitrate: Option<u64>,
) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants SET
                width = COALESCE(?1, width),
                height = COALESCE(?2, height),
                duration_ms = COALESCE(?3, duration_ms),
                bitrate = COALESCE(?4, bitrate)
             WHERE id = ?5",
            params![
                width,
                height,
                duration_ms.map(|d| d as i64),
                bitrate.map(|b| b as i64),
                variant_id.to_string(),
            ],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

fn update_variant_page_count(
    ctx: &AppContext,
    variant_id: VariantId,
    page_count: u32,
) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE media_variants SET page_count = ?1 WHERE id = ?2",
            params![page_count, variant_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_new_item_and_variant(
    ctx: &AppContext,
    title: &str,
    media_type: MediaType,
    format: &str,
    root: &LibraryRoot,
    path_str: &str,
    mime_type: &str,
    file_size: u64,
    mtime: Option<i64>,
    content_hash: &str,
    now: &str,
) -> Result<(ItemId, VariantId)> {
    let item_id = ItemId::new();
    let variant_id = VariantId::new();

    let mut conn = ctx.db.connection();
    let tx = conn.transaction().map_err(database::DatabaseError::from)?;
    tx.execute(
        "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at, safety_status, visibility_state)
         VALUES (?1, ?2, ?3, 'unrated', ?4, ?4, 'unreviewed', 'visible')",
        params![item_id.to_string(), media_type_to_str(media_type), title, now],
    )
    .map_err(database::DatabaseError::from)?;
    tx.execute(
        "INSERT INTO media_variants
             (id, item_id, mime_type, format, local_path, file_size, mtime_unix, content_hash, library_root_id, last_seen_at, download_permitted, cache_permitted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, 1)",
        params![
            variant_id.to_string(),
            item_id.to_string(),
            mime_type,
            format,
            path_str,
            file_size as i64,
            mtime,
            content_hash,
            root.id.to_string(),
            now,
        ],
    )
    .map_err(database::DatabaseError::from)?;
    tx.commit().map_err(database::DatabaseError::from)?;

    Ok((item_id, variant_id))
}

fn mark_missing(ctx: &AppContext, root_id: LibraryRootId, scan_started_at: &str) -> Result<u32> {
    let affected = ctx
        .db
        .connection()
        .execute(
            "UPDATE media_variants SET local_path = NULL
             WHERE library_root_id = ?1 AND local_path IS NOT NULL
               AND (last_seen_at IS NULL OR last_seen_at < ?2)",
            params![root_id.to_string(), scan_started_at],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(affected as u32)
}

fn update_root_last_scanned(ctx: &AppContext, root_id: LibraryRootId, now: &str) -> Result<()> {
    ctx.db
        .connection()
        .execute(
            "UPDATE library_roots SET last_scanned_at = ?1 WHERE id = ?2",
            params![now, root_id.to_string()],
        )
        .map_err(database::DatabaseError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanning_adds_new_files_and_classifies_them() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clip.mp4"), b"fake video bytes").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let report = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(report.roots.len(), 1);
        let root_result = &report.roots[0];
        assert_eq!(root_result.added, 1);
        assert_eq!(root_result.files_seen, 1);
        assert!(root_result.error.is_none());

        let count: i64 = ctx
            .db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE media_type = 'video'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn rescanning_unchanged_files_is_idempotent() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pic.png"), b"fake png bytes").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let first = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(first.roots[0].added, 1);

        let second = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(second.roots[0].added, 0);
        assert_eq!(second.roots[0].updated, 0);
        assert_eq!(second.roots[0].files_seen, 1);

        let count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn moving_a_file_preserves_item_identity_and_favorite() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("photo.jpg");
        std::fs::write(&original, b"unique content for move test").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let first = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(first.roots[0].added, 1);

        let item_id: String = ctx
            .db
            .connection()
            .query_row("SELECT id FROM media_items", [], |r| r.get(0))
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, favorite) VALUES (?1, 1)",
                params![item_id],
            )
            .unwrap();

        let subfolder = dir.path().join("subfolder");
        std::fs::create_dir_all(&subfolder).unwrap();
        let new_path = subfolder.join("photo.jpg");
        std::fs::rename(&original, &new_path).unwrap();

        let second = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(
            second.roots[0].moved, 1,
            "expected the relocated file to be detected as a move"
        );
        assert_eq!(second.roots[0].added, 0);

        let item_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(item_count, 1, "move must not create a duplicate item");

        let favorite: bool = ctx
            .db
            .connection()
            .query_row(
                "SELECT favorite FROM user_state WHERE item_id = ?1",
                params![item_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(favorite, "favorite must survive a file move");
    }

    #[test]
    fn deleting_a_file_marks_it_missing_without_deleting_the_item() {
        // A real filesystem data_dir: this file classifies as a Story
        // (`.txt`), which triggers StoryService::ensure and writes a
        // cache file — open_in_memory()'s `:memory:` placeholder path
        // would create a literal `./:memory:/` directory on disk.
        let data_dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(data_dir.path()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("story.txt");
        std::fs::write(&path, b"once upon a time").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        ScanService::scan_path(&ctx, dir.path()).unwrap();
        std::fs::remove_file(&path).unwrap();

        let report = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(report.roots[0].missing, 1);

        let conn = ctx.db.connection();
        let item_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        let local_path: Option<String> = conn
            .query_row("SELECT local_path FROM media_variants LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(item_count, 1, "missing file must not delete the item");
        assert_eq!(local_path, None);
    }

    #[test]
    fn unsupported_extensions_are_skipped_not_fatal() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.xyz"), b"???").unwrap();
        std::fs::write(dir.path().join("clip.mp4"), b"video bytes").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let report = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(report.roots[0].added, 1);
        assert_eq!(report.roots[0].unsupported, 1);
        assert_eq!(report.skipped_total, 1);
        assert_eq!(
            report.skipped[0].path,
            dir.path().join("notes.xyz").display().to_string()
        );
    }

    #[test]
    fn scan_path_requires_a_registered_root() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let err = ScanService::scan_path(&ctx, dir.path()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn scanning_a_cbz_populates_page_count_and_a_thumbnail() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;

        let data_dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(data_dir.path()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cbz_path = dir.path().join("book.cbz");

        let mut page_png = Vec::new();
        let img = image::RgbImage::from_pixel(64, 48, image::Rgb([1, 2, 3]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut page_png),
                image::ImageFormat::Png,
            )
            .unwrap();

        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        writer.start_file("000.png", options).unwrap();
        writer.write_all(&page_png).unwrap();
        writer.start_file("001.png", options).unwrap();
        writer.write_all(&page_png).unwrap();
        writer.finish().unwrap();

        LibraryRootService::add(&ctx, dir.path(), None).unwrap();
        let report = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(report.roots[0].added, 1);

        let (page_count, variant_id_str): (Option<i64>, String) = ctx
            .db
            .connection()
            .query_row(
                "SELECT page_count, id FROM media_variants LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(page_count, Some(2));

        let variant_id = VariantId(Uuid::parse_str(&variant_id_str).unwrap());
        let thumb_path = ThumbnailService::cache_path(&ctx, variant_id);
        assert!(
            thumb_path.exists(),
            "expected a comic-page thumbnail to be generated"
        );
    }

    #[test]
    fn scanning_a_markdown_story_persists_a_story_document() {
        let data_dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(data_dir.path()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tale.md"), "# Once\nUpon a time.\n").unwrap();
        LibraryRootService::add(&ctx, dir.path(), None).unwrap();

        let report = ScanService::scan_path(&ctx, dir.path()).unwrap();
        assert_eq!(report.roots[0].added, 1);

        let item_id_str: String = ctx
            .db
            .connection()
            .query_row("SELECT id FROM media_items", [], |r| r.get(0))
            .unwrap();
        let item_id = ItemId(Uuid::parse_str(&item_id_str).unwrap());

        let doc = StoryService::get(&ctx, item_id).unwrap();
        assert!(
            doc.is_some(),
            "expected a story_documents row after scanning a .md file"
        );
        let content = StoryService::read_content(&ctx, item_id).unwrap();
        assert!(content.contains("Upon a time."));
    }

    #[test]
    fn scan_all_skips_disabled_roots() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clip.mp4"), b"video").unwrap();
        let root = LibraryRootService::add(&ctx, dir.path(), None).unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE library_roots SET enabled = 0 WHERE id = ?1",
                params![root.id.to_string()],
            )
            .unwrap();

        let report = ScanService::scan_all(&ctx).unwrap();
        assert_eq!(report.roots.len(), 0);
    }
}
