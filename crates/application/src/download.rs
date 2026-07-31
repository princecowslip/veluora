//! Downloads and offline use (Workstream 11) — Milestone J.
//!
//! There is no persistent background daemon anywhere in this codebase:
//! the CLI is one-shot, and `local-api`/the GUI are the only long-lived
//! processes, each opening its own connection to the same shared
//! `veloura.db` file. Pause/resume/cancel are therefore coordinated
//! through that shared SQLite row (an atomic claim + a `state` column
//! polled between chunks), not an in-memory flag — a `pause` issued
//! from a second, independent CLI invocation genuinely stops a
//! transfer running in a first, unrelated process.
//!
//! `<data_dir>/downloads` holds only fully verified, finalized files —
//! nothing is ever created there except by an atomic
//! `std::fs::rename` (via `tokio::fs::rename`) of a fully streamed and
//! (when possible) checksum-verified temp file under
//! `<data_dir>/temp/downloads`. That's the whole reason partial
//! downloads never appear as complete.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use domain::{
    BlockCandidate, ChecksumState, Download, DownloadId, DownloadState, ItemId, SourceId, VariantId,
};
use futures::StreamExt;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::privacy::PrivacyService;
use crate::settings::SettingsService;
use crate::source::SourceService;
use crate::time_format::{from_rfc3339, to_rfc3339};

const SELECT_DOWNLOAD_COLUMNS: &str = "SELECT id, item_id, variant_id, state, destination, \
     bytes_total, bytes_received, checksum_state, retry_count, created_at, started_at, \
     completed_at, failure_code, source_id, pinned, temp_path, expected_checksum, \
     checksum_algorithm, etag, last_modified, updated_at FROM downloads";

/// Whether [`DownloadService::add`] would accept a given item/variant —
/// checked eagerly by `add` (which rejects with the same reasons) and
/// exposed separately so callers (GUI/CLI/TUI) can gate a "Download"
/// action without attempting one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityReport {
    pub eligible: bool,
    pub reasons: Vec<String>,
}

/// A [`Download`] enriched with display fields no download-list UI
/// would otherwise be able to show without an extra per-row fetch.
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadSummary {
    #[serde(flatten)]
    pub download: Download,
    pub item_title: String,
    pub source_display_name: Option<String>,
}

struct VariantRow {
    item_id: ItemId,
    source_ref_id: Option<String>,
    local_path: Option<String>,
    remote_url: Option<String>,
    mime_type: String,
    file_size: Option<u64>,
    checksum: Option<String>,
    download_permitted: bool,
}

pub struct DownloadService;

impl DownloadService {
    /// Checks: the variant exists and belongs to the item, is marked
    /// `download_permitted` (ADR-007), isn't already local, has a
    /// remote URL, its source is enabled with a downloads-capable
    /// connector, the item isn't matched by an enabled block rule, the
    /// downloads directory is writable, and — when both a quota and a
    /// declared file size are known — the download wouldn't exceed it.
    pub fn check_eligibility(
        ctx: &AppContext,
        item_id: ItemId,
        variant_id: VariantId,
    ) -> Result<EligibilityReport> {
        let mut reasons = Vec::new();

        let Some(variant) = Self::find_variant(ctx, variant_id)? else {
            return Ok(EligibilityReport {
                eligible: false,
                reasons: vec!["variant not found".to_string()],
            });
        };
        if variant.item_id != item_id {
            return Ok(EligibilityReport {
                eligible: false,
                reasons: vec!["variant does not belong to this item".to_string()],
            });
        }

        if !variant.download_permitted {
            reasons.push("the source has not marked this variant as downloadable".to_string());
        }
        if variant.local_path.is_some() {
            reasons.push("this variant is already local".to_string());
        }
        if variant.remote_url.is_none() {
            reasons.push("this variant has no remote URL to fetch".to_string());
        }

        let source_id = match &variant.source_ref_id {
            Some(source_ref_id) => Self::source_id_for_source_ref(ctx, source_ref_id)?,
            None => None,
        };
        match source_id {
            Some(source_id) => match SourceService::connector_for(ctx, source_id) {
                Ok((source, connector)) => {
                    if !source.enabled {
                        reasons.push("the source is disabled".to_string());
                    }
                    if !connector.capabilities().downloads {
                        reasons.push("the connector does not support downloads".to_string());
                    }
                }
                Err(_) => reasons.push("the source no longer exists".to_string()),
            },
            None => reasons.push("this variant has no originating source".to_string()),
        }

        if Self::is_blocked(ctx, item_id, source_id)? {
            reasons.push("this item is blocked by an enabled block rule".to_string());
        }

        let downloads_dir = ctx.data_dir.join("downloads");
        if fs::create_dir_all(&downloads_dir).is_err() {
            reasons.push("the downloads directory is not writable".to_string());
        }

        if let (Some(quota), Some(size)) = (
            SettingsService::download_quota_bytes(ctx)?,
            variant.file_size,
        ) {
            let current = PrivacyService::download_directory_size_bytes(ctx)?;
            if current + size > quota {
                reasons.push("this download would exceed the configured quota".to_string());
            }
        }

        Ok(EligibilityReport {
            eligible: reasons.is_empty(),
            reasons,
        })
    }

    /// Validates eligibility, renders a naming-template destination,
    /// and inserts a `Queued` row. Does not fetch any bytes — callers
    /// invoke [`Self::run`]/[`Self::resume`] separately (see the
    /// module doc comment for why the split exists).
    pub fn add(ctx: &AppContext, item_id: ItemId, variant_id: VariantId) -> Result<Download> {
        let report = Self::check_eligibility(ctx, item_id, variant_id)?;
        if !report.eligible {
            return Err(AppError::UnsupportedCapability(report.reasons.join("; ")));
        }
        let variant = Self::find_variant(ctx, variant_id)?
            .ok_or_else(|| AppError::NotFound(format!("variant {variant_id}")))?;
        let source_id = match &variant.source_ref_id {
            Some(source_ref_id) => Self::source_id_for_source_ref(ctx, source_ref_id)?,
            None => None,
        };

        let item_title = Self::item_title(ctx, item_id)?;
        let source_display_name = match source_id {
            Some(sid) => Self::source_display_name(ctx, sid)?,
            None => None,
        };
        let ext = extension_for(&variant.mime_type, variant.remote_url.as_deref());
        let relative = render_naming_template(
            &SettingsService::download_naming_template(ctx)?,
            &item_title,
            source_display_name.as_deref().unwrap_or("unknown-source"),
            item_id,
            source_id,
            &ext,
            1,
        );
        let destination = ctx.data_dir.join("downloads").join(relative);

        let now = time::OffsetDateTime::now_utc();
        let download = Download {
            id: DownloadId::new(),
            item_id,
            variant_id,
            state: DownloadState::Queued,
            destination: destination.to_string_lossy().into_owned(),
            bytes_total: variant.file_size,
            bytes_received: 0,
            checksum_state: ChecksumState::Pending,
            retry_count: 0,
            created_at: now,
            started_at: None,
            completed_at: None,
            failure_code: None,
            source_id,
            pinned: false,
            temp_path: None,
            expected_checksum: variant.checksum,
            checksum_algorithm: None,
            etag: None,
            last_modified: None,
            updated_at: Some(now),
        };
        Self::insert(ctx, &download)?;
        Ok(download)
    }

    pub fn list(ctx: &AppContext, item_id: Option<ItemId>) -> Result<Vec<DownloadSummary>> {
        let mut downloads = Vec::new();
        {
            let conn = ctx.db.connection();
            match item_id {
                Some(item_id) => {
                    let mut stmt = conn
                        .prepare(&format!(
                            "{SELECT_DOWNLOAD_COLUMNS} WHERE item_id = ?1 ORDER BY created_at DESC"
                        ))
                        .map_err(database::DatabaseError::from)?;
                    let rows = stmt
                        .query_map(params![item_id.to_string()], row_to_download)
                        .map_err(database::DatabaseError::from)?;
                    for row in rows {
                        downloads.push(row.map_err(database::DatabaseError::from)?);
                    }
                }
                None => {
                    let mut stmt = conn
                        .prepare(&format!(
                            "{SELECT_DOWNLOAD_COLUMNS} ORDER BY created_at DESC"
                        ))
                        .map_err(database::DatabaseError::from)?;
                    let rows = stmt
                        .query_map([], row_to_download)
                        .map_err(database::DatabaseError::from)?;
                    for row in rows {
                        downloads.push(row.map_err(database::DatabaseError::from)?);
                    }
                }
            }
        }

        let mut out = Vec::with_capacity(downloads.len());
        for download in downloads {
            let item_title = Self::item_title(ctx, download.item_id)?;
            let source_display_name = match download.source_id {
                Some(sid) => Self::source_display_name(ctx, sid)?,
                None => None,
            };
            out.push(DownloadSummary {
                item_title,
                source_display_name,
                download,
            });
        }
        Ok(out)
    }

    pub fn find(ctx: &AppContext, id: DownloadId) -> Result<Option<Download>> {
        let conn = ctx.db.connection();
        conn.query_row(
            &format!("{SELECT_DOWNLOAD_COLUMNS} WHERE id = ?1"),
            params![id.to_string()],
            row_to_download,
        )
        .optional()
        .map_err(|e| database::DatabaseError::from(e).into())
    }

    /// A plain state flip. Whatever process/task is currently running
    /// [`Self::run`] for this id notices on its next per-chunk poll
    /// and stops within one network round trip.
    pub fn pause(ctx: &AppContext, id: DownloadId) -> Result<()> {
        if Self::find(ctx, id)?.is_none() {
            return Err(AppError::NotFound(format!("download {id}")));
        }
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'paused', updated_at = ?2 \
                 WHERE id = ?1 AND state IN ('queued', 'active')",
                params![id.to_string(), now],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    /// Also deletes the `.part` temp file eagerly, so a cancel from a
    /// second process frees disk space immediately rather than waiting
    /// for a running loop in a first process to notice.
    pub fn cancel(ctx: &AppContext, id: DownloadId) -> Result<()> {
        let download =
            Self::find(ctx, id)?.ok_or_else(|| AppError::NotFound(format!("download {id}")))?;
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'canceled', updated_at = ?2 \
                 WHERE id = ?1 AND state NOT IN ('completed', 'canceled')",
                params![id.to_string(), now],
            )
            .map_err(database::DatabaseError::from)?;
        if let Some(temp_path) = &download.temp_path {
            let _ = fs::remove_file(temp_path);
        }
        Ok(())
    }

    pub fn set_pinned(ctx: &AppContext, id: DownloadId, pinned: bool) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute(
                "UPDATE downloads SET pinned = ?2 WHERE id = ?1",
                params![id.to_string(), pinned as i64],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("download {id}")));
        }
        Ok(())
    }

    /// Deletes the `downloads` row. When `delete_file` is true and the
    /// download is `Completed`, also deletes the permanent file and
    /// clears the variant's `local_path`/`file_size`/`checksum`. The
    /// media item, source reference, favorites, notes, and collection
    /// membership are never touched either way — this is what
    /// "deleting a download can preserve the library reference" means
    /// concretely.
    pub fn remove(ctx: &AppContext, id: DownloadId, delete_file: bool) -> Result<()> {
        let download =
            Self::find(ctx, id)?.ok_or_else(|| AppError::NotFound(format!("download {id}")))?;
        if delete_file && download.state == DownloadState::Completed {
            let _ = fs::remove_file(&download.destination);
            ctx.db
                .connection()
                .execute(
                    "UPDATE media_variants SET local_path = NULL, file_size = NULL, checksum = NULL \
                     WHERE id = ?1 AND local_path = ?2",
                    params![download.variant_id.to_string(), download.destination],
                )
                .map_err(database::DatabaseError::from)?;
        }
        if let Some(temp_path) = &download.temp_path {
            let _ = fs::remove_file(temp_path);
        }
        ctx.db
            .connection()
            .execute(
                "DELETE FROM downloads WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    /// Drives one download to a terminal (or paused/canceled) state.
    /// Atomically claims the row so two concurrent callers can't both
    /// stream into the same temp file — a no-op (returns the current
    /// row unchanged) if the claim fails because another runner
    /// already owns it, or it's already in a terminal state.
    pub async fn run(ctx: &AppContext, id: DownloadId) -> Result<Download> {
        if !Self::claim(ctx, id)? {
            return Self::find(ctx, id)?
                .ok_or_else(|| AppError::NotFound(format!("download {id}")));
        }

        let mut download =
            Self::find(ctx, id)?.ok_or_else(|| AppError::NotFound(format!("download {id}")))?;
        if let Err(e) = Self::run_inner(ctx, &mut download).await {
            Self::mark_paused_on_error(ctx, id, &e.to_string())?;
        }

        Self::find(ctx, id)?.ok_or_else(|| AppError::NotFound(format!("download {id}")))
    }

    /// A documented alias for [`Self::run`] — resuming *is* re-entering
    /// the same claim-and-fetch loop from wherever `bytes_received`/
    /// `temp_path` left off.
    pub async fn resume(ctx: &AppContext, id: DownloadId) -> Result<Download> {
        Self::run(ctx, id).await
    }

    /// Deletes `.part` files under `<data_dir>/temp/downloads/` with no
    /// matching active/paused/queued row — orphaned by a crash before
    /// the row existed, or left behind after the row was removed
    /// without touching the file. Explicit, not run automatically.
    pub fn sweep_orphaned_temp_files(ctx: &AppContext) -> Result<u64> {
        let temp_dir = ctx.data_dir.join("temp").join("downloads");
        if !temp_dir.exists() {
            return Ok(0);
        }
        let active_ids: HashSet<String> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare("SELECT id FROM downloads WHERE state IN ('active', 'paused', 'queued')")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(database::DatabaseError::from)?;
            let mut set = HashSet::new();
            for row in rows {
                set.insert(row.map_err(database::DatabaseError::from)?);
            }
            set
        };

        let mut swept = 0u64;
        for entry in fs::read_dir(&temp_dir)?.filter_map(|e| e.ok()) {
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if !active_ids.contains(stem) && fs::remove_file(&path).is_ok() {
                swept += 1;
            }
        }
        Ok(swept)
    }

    // --- internals ---

    fn claim(ctx: &AppContext, id: DownloadId) -> Result<bool> {
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        let affected = ctx
            .db
            .connection()
            .execute(
                "UPDATE downloads SET \
                     state = 'active', \
                     retry_count = retry_count + CASE WHEN state = 'failed' THEN 1 ELSE 0 END, \
                     started_at = COALESCE(started_at, ?2), \
                     updated_at = ?2 \
                 WHERE id = ?1 AND state IN ('queued', 'paused', 'failed')",
                params![id.to_string(), now],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(affected > 0)
    }

    async fn run_inner(ctx: &AppContext, download: &mut Download) -> Result<()> {
        let variant = Self::find_variant(ctx, download.variant_id)?
            .ok_or_else(|| AppError::NotFound(format!("variant {}", download.variant_id)))?;
        let remote_url = variant.remote_url.clone().ok_or_else(|| {
            AppError::UnsupportedCapability("variant has no remote URL".to_string())
        })?;

        let temp_dir = ctx.data_dir.join("temp").join("downloads");
        tokio::fs::create_dir_all(&temp_dir).await?;
        let temp_path = temp_dir.join(format!("{}.part", download.id));
        Self::set_temp_path(ctx, download.id, Some(&temp_path.to_string_lossy()))?;
        download.temp_path = Some(temp_path.to_string_lossy().into_owned());

        let existing_len = tokio::fs::metadata(&temp_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let client = reqwest::Client::new();
        let mut request = client.get(&remote_url);
        if existing_len > 0 {
            request = request.header("Range", format!("bytes={existing_len}-"));
            if let Some(etag) = &download.etag {
                request = request.header("If-Range", etag.clone());
            } else if let Some(last_modified) = &download.last_modified {
                request = request.header("If-Range", last_modified.clone());
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AppError::Network(format!(
                "unexpected response status: {}",
                response.status()
            )));
        }

        let resumed = existing_len > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let last_modified = response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let content_length = response.content_length();
        let bytes_total = if resumed {
            content_length.map(|len| existing_len + len)
        } else {
            content_length
        };

        let mut file = if resumed {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&temp_path)
                .await?
        } else {
            tokio::fs::File::create(&temp_path).await?
        };
        let mut bytes_received = if resumed { existing_len } else { 0 };

        download.etag = etag.clone();
        download.last_modified = last_modified.clone();
        download.bytes_total = bytes_total;
        Self::update_progress(
            ctx,
            download.id,
            bytes_received,
            bytes_total,
            &etag,
            &last_modified,
        )?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::Network(e.to_string()))?;
            file.write_all(&chunk).await?;
            bytes_received += chunk.len() as u64;
            Self::update_progress(
                ctx,
                download.id,
                bytes_received,
                bytes_total,
                &etag,
                &last_modified,
            )?;

            match Self::current_state(ctx, download.id)? {
                Some(DownloadState::Paused) => {
                    file.flush().await?;
                    download.bytes_received = bytes_received;
                    download.state = DownloadState::Paused;
                    return Ok(());
                }
                Some(DownloadState::Canceled) => {
                    drop(file);
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    download.bytes_received = bytes_received;
                    download.state = DownloadState::Canceled;
                    download.temp_path = None;
                    return Ok(());
                }
                _ => {}
            }
        }
        file.flush().await?;
        drop(file);

        if let Some(total) = bytes_total {
            if bytes_received != total {
                return Err(AppError::Network(format!(
                    "downloaded {bytes_received} bytes, expected {total}"
                )));
            }
        }

        let computed_checksum = hash_file(&temp_path)?;
        let checksum_state = match &download.expected_checksum {
            Some(expected) if expected == &computed_checksum => ChecksumState::Verified,
            Some(_) => ChecksumState::Mismatch,
            None => ChecksumState::Unavailable,
        };
        if checksum_state == ChecksumState::Mismatch {
            let _ = tokio::fs::remove_file(&temp_path).await;
            Self::mark_failed(
                ctx,
                download.id,
                "checksum mismatch",
                ChecksumState::Mismatch,
            )?;
            download.state = DownloadState::Failed;
            download.checksum_state = ChecksumState::Mismatch;
            download.temp_path = None;
            return Ok(());
        }

        let final_path = unique_destination(Path::new(&download.destination));
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&temp_path, &final_path).await?;

        Self::finalize(
            ctx,
            download,
            &final_path,
            bytes_received,
            &computed_checksum,
        )?;
        Ok(())
    }

    fn finalize(
        ctx: &AppContext,
        download: &mut Download,
        final_path: &Path,
        bytes_received: u64,
        checksum: &str,
    ) -> Result<()> {
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        let destination = final_path.to_string_lossy().into_owned();
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'completed', destination = ?2, bytes_received = ?3, \
                 bytes_total = ?3, checksum_state = 'verified', checksum_algorithm = 'blake3', \
                 completed_at = ?4, updated_at = ?4, temp_path = NULL WHERE id = ?1",
                params![
                    download.id.to_string(),
                    destination,
                    bytes_received as i64,
                    now
                ],
            )
            .map_err(database::DatabaseError::from)?;
        // The row's checksum_state above is only correct for the
        // `Verified`/`Unavailable` cases that reach this point (a
        // `Mismatch` never gets here — see `run_inner`) — patch it
        // back to `Unavailable` when there was nothing to verify
        // against, so the UI doesn't claim a check that never happened.
        if download.checksum_state != ChecksumState::Verified {
            ctx.db
                .connection()
                .execute(
                    "UPDATE downloads SET checksum_state = 'unavailable' WHERE id = ?1 AND ?2 IS NULL",
                    params![download.id.to_string(), download.expected_checksum],
                )
                .map_err(database::DatabaseError::from)?;
        }

        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET local_path = ?2, file_size = ?3, checksum = ?4 WHERE id = ?1",
                params![
                    download.variant_id.to_string(),
                    destination,
                    bytes_received as i64,
                    checksum,
                ],
            )
            .map_err(database::DatabaseError::from)?;

        download.state = DownloadState::Completed;
        download.destination = destination;
        download.bytes_received = bytes_received;
        download.bytes_total = Some(bytes_received);
        download.checksum_state = if download.expected_checksum.is_some() {
            ChecksumState::Verified
        } else {
            ChecksumState::Unavailable
        };
        download.checksum_algorithm = Some("blake3".to_string());
        download.temp_path = None;
        Ok(())
    }

    fn mark_paused_on_error(ctx: &AppContext, id: DownloadId, message: &str) -> Result<()> {
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'paused', failure_code = ?2, updated_at = ?3 \
                 WHERE id = ?1 AND state NOT IN ('canceled', 'completed')",
                params![id.to_string(), message, now],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    fn mark_failed(
        ctx: &AppContext,
        id: DownloadId,
        message: &str,
        checksum_state: ChecksumState,
    ) -> Result<()> {
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'failed', failure_code = ?2, checksum_state = ?3, \
                 updated_at = ?4, temp_path = NULL WHERE id = ?1",
                params![
                    id.to_string(),
                    message,
                    checksum_state_to_str(checksum_state),
                    now,
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    fn current_state(ctx: &AppContext, id: DownloadId) -> Result<Option<DownloadState>> {
        let conn = ctx.db.connection();
        conn.query_row(
            "SELECT state FROM downloads WHERE id = ?1",
            params![id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| -> AppError { database::DatabaseError::from(e).into() })
        .map(|opt| opt.map(|s| state_from_str(&s)))
    }

    fn update_progress(
        ctx: &AppContext,
        id: DownloadId,
        bytes_received: u64,
        bytes_total: Option<u64>,
        etag: &Option<String>,
        last_modified: &Option<String>,
    ) -> Result<()> {
        let now = to_rfc3339(time::OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET bytes_received = ?2, bytes_total = ?3, etag = ?4, \
                 last_modified = ?5, updated_at = ?6 WHERE id = ?1",
                params![
                    id.to_string(),
                    bytes_received as i64,
                    bytes_total.map(|n| n as i64),
                    etag,
                    last_modified,
                    now,
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    fn set_temp_path(ctx: &AppContext, id: DownloadId, temp_path: Option<&str>) -> Result<()> {
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET temp_path = ?2 WHERE id = ?1",
                params![id.to_string(), temp_path],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    fn find_variant(ctx: &AppContext, variant_id: VariantId) -> Result<Option<VariantRow>> {
        let conn = ctx.db.connection();
        conn.query_row(
            "SELECT item_id, source_ref_id, local_path, remote_url, mime_type, file_size, checksum, download_permitted \
             FROM media_variants WHERE id = ?1",
            params![variant_id.to_string()],
            |row| {
                let item_id: String = row.get(0)?;
                Ok(VariantRow {
                    item_id: ItemId(parse_uuid(&item_id)?),
                    source_ref_id: row.get(1)?,
                    local_path: row.get(2)?,
                    remote_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    file_size: row.get::<_, Option<i64>>(5)?.map(|n| n as u64),
                    checksum: row.get(6)?,
                    download_permitted: row.get::<_, i64>(7)? != 0,
                })
            },
        )
        .optional()
        .map_err(|e| database::DatabaseError::from(e).into())
    }

    fn source_id_for_source_ref(ctx: &AppContext, source_ref_id: &str) -> Result<Option<SourceId>> {
        let conn = ctx.db.connection();
        let source_id: Option<String> = conn
            .query_row(
                "SELECT source_id FROM source_references WHERE id = ?1",
                params![source_ref_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database::DatabaseError::from)?;
        Ok(source_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(SourceId))
    }

    fn is_blocked(ctx: &AppContext, item_id: ItemId, source_id: Option<SourceId>) -> Result<bool> {
        let rules: Vec<domain::BlockRule> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare(
                    "SELECT id, rule_type, target, scope, reason, created_at, enabled \
                     FROM block_rules WHERE enabled = 1",
                )
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], row_to_block_rule)
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(database::DatabaseError::from)?);
            }
            out
        };
        if rules.is_empty() {
            return Ok(false);
        }

        let tag_values = Self::tag_values_for_item(ctx, item_id)?;
        let candidate = BlockCandidate {
            item_id,
            tag_values,
            creator_ids: Vec::new(),
            source_id: source_id.map(|s| s.to_string()),
        };
        Ok(rules.iter().any(|rule| rule.evaluate(&candidate)))
    }

    fn tag_values_for_item(ctx: &AppContext, item_id: ItemId) -> Result<Vec<String>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT t.normalized_value FROM media_item_tags mit \
                 JOIN tags t ON t.id = mit.tag_id WHERE mit.item_id = ?1",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map(params![item_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(database::DatabaseError::from)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(out)
    }

    fn item_title(ctx: &AppContext, item_id: ItemId) -> Result<String> {
        let conn = ctx.db.connection();
        let title: Option<String> = conn
            .query_row(
                "SELECT title FROM media_items WHERE id = ?1",
                params![item_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(database::DatabaseError::from)?;
        Ok(title.unwrap_or_else(|| "(unknown item)".to_string()))
    }

    fn source_display_name(ctx: &AppContext, source_id: SourceId) -> Result<Option<String>> {
        let conn = ctx.db.connection();
        conn.query_row(
            "SELECT display_name FROM sources WHERE id = ?1",
            params![source_id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| database::DatabaseError::from(e).into())
    }

    fn insert(ctx: &AppContext, download: &Download) -> Result<()> {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO downloads (id, item_id, variant_id, state, destination, bytes_total, \
                 bytes_received, checksum_state, retry_count, created_at, started_at, completed_at, \
                 failure_code, source_id, pinned, temp_path, expected_checksum, checksum_algorithm, \
                 etag, last_modified, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                params![
                    download.id.to_string(),
                    download.item_id.to_string(),
                    download.variant_id.to_string(),
                    state_to_str(download.state),
                    download.destination,
                    download.bytes_total.map(|n| n as i64),
                    download.bytes_received as i64,
                    checksum_state_to_str(download.checksum_state),
                    download.retry_count,
                    to_rfc3339(download.created_at),
                    download.started_at.map(to_rfc3339),
                    download.completed_at.map(to_rfc3339),
                    download.failure_code,
                    download.source_id.map(|s| s.to_string()),
                    download.pinned as i64,
                    download.temp_path,
                    download.expected_checksum,
                    download.checksum_algorithm,
                    download.etag,
                    download.last_modified,
                    download.updated_at.map(to_rfc3339),
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Picks an extension from the declared MIME type first, falling back
/// to the URL's own extension, falling back to `bin`.
fn extension_for(mime_type: &str, url: Option<&str>) -> String {
    if let Some(ext) = mime_guess::get_mime_extensions_str(mime_type).and_then(|exts| exts.first())
    {
        return (*ext).to_string();
    }
    if let Some(url) = url {
        let path_part = url.split(['?', '#']).next().unwrap_or(url);
        if let Some(ext) = Path::new(path_part)
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty())
        {
            return ext.to_string();
        }
    }
    "bin".to_string()
}

/// Strips characters that are unsafe or meaningless as a single path
/// component: path separators (defeats traversal — a template value
/// can never introduce a new directory level), NUL/control characters,
/// and Windows-reserved punctuation. Trims trailing dots/spaces
/// (Windows-hostile) and caps length. Never returns an empty string.
fn sanitize_path_component(input: &str) -> String {
    let mut sanitized: String = input
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    sanitized = sanitized.trim_end_matches(['.', ' ']).trim().to_string();
    if sanitized.len() > 150 {
        let mut cut = 150;
        while !sanitized.is_char_boundary(cut) {
            cut -= 1;
        }
        sanitized.truncate(cut);
    }
    if sanitized.is_empty() {
        sanitized = "untitled".to_string();
    }
    sanitized
}

/// Renders `template`'s `{title}`/`{source}`/`{source_id}`/`{item_id}`/
/// `{sequence}`/`{ext}` tokens. Only the substituted *values* are
/// sanitized — the template's own `/` separators are preserved so it
/// can still describe a directory structure.
fn render_naming_template(
    template: &str,
    item_title: &str,
    source_display_name: &str,
    item_id: ItemId,
    source_id: Option<SourceId>,
    ext: &str,
    sequence: u32,
) -> PathBuf {
    let rendered = template
        .replace("{title}", &sanitize_path_component(item_title))
        .replace("{source}", &sanitize_path_component(source_display_name))
        .replace(
            "{source_id}",
            &source_id
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        )
        .replace("{item_id}", &item_id.to_string())
        .replace("{sequence}", &sequence.to_string())
        .replace("{ext}", &sanitize_path_component(ext));
    PathBuf::from(rendered)
}

/// If `path` already exists, appends `-2`, `-3`, ... before the
/// extension until a free name is found. Checked at finalize time, not
/// at queue time — claiming a filename slot for a download that might
/// fail or be canceled would be wrong.
fn unique_destination(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let ext = path.extension().and_then(|s| s.to_str());
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut n = 2u32;
    loop {
        let candidate_name = match ext {
            Some(ext) => format!("{stem}-{n}.{ext}"),
            None => format!("{stem}-{n}"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

fn parse_uuid(s: &str) -> rusqlite::Result<uuid::Uuid> {
    uuid::Uuid::parse_str(s).map_err(|_| {
        rusqlite::Error::InvalidColumnType(0, "id".into(), rusqlite::types::Type::Text)
    })
}

fn row_to_download(row: &Row) -> rusqlite::Result<Download> {
    let id: String = row.get(0)?;
    let item_id: String = row.get(1)?;
    let variant_id: String = row.get(2)?;
    let state: String = row.get(3)?;
    let checksum_state: String = row.get(7)?;
    let created_at: String = row.get(9)?;
    let started_at: Option<String> = row.get(10)?;
    let completed_at: Option<String> = row.get(11)?;
    let source_id: Option<String> = row.get(13)?;
    let updated_at: Option<String> = row.get(20)?;

    Ok(Download {
        id: DownloadId(parse_uuid(&id)?),
        item_id: ItemId(parse_uuid(&item_id)?),
        variant_id: VariantId(parse_uuid(&variant_id)?),
        state: state_from_str(&state),
        destination: row.get(4)?,
        bytes_total: row.get::<_, Option<i64>>(5)?.map(|n| n as u64),
        bytes_received: row.get::<_, i64>(6)? as u64,
        checksum_state: checksum_state_from_str(&checksum_state),
        retry_count: row.get::<_, i64>(8)? as u32,
        created_at: from_rfc3339(&created_at).unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
        started_at: started_at.and_then(|s| from_rfc3339(&s)),
        completed_at: completed_at.and_then(|s| from_rfc3339(&s)),
        failure_code: row.get(12)?,
        source_id: source_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(SourceId),
        pinned: row.get::<_, i64>(14)? != 0,
        temp_path: row.get(15)?,
        expected_checksum: row.get(16)?,
        checksum_algorithm: row.get(17)?,
        etag: row.get(18)?,
        last_modified: row.get(19)?,
        updated_at: updated_at.and_then(|s| from_rfc3339(&s)),
    })
}

fn row_to_block_rule(row: &Row) -> rusqlite::Result<domain::BlockRule> {
    let id: String = row.get(0)?;
    let rule_type: String = row.get(1)?;
    let scope: String = row.get(3)?;
    let created_at: String = row.get(5)?;
    Ok(domain::BlockRule {
        id: domain::BlockRuleId(parse_uuid(&id)?),
        rule_type: rule_type_from_str(&rule_type),
        target: row.get(2)?,
        scope: scope_from_str(&scope),
        reason: row.get(4)?,
        created_at: from_rfc3339(&created_at).unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
        enabled: row.get::<_, i64>(6)? != 0,
    })
}

fn rule_type_from_str(s: &str) -> domain::RuleType {
    use domain::RuleType::*;
    match s {
        "exact_item" => ExactItem,
        "source" => Source,
        "creator" => Creator,
        "series" => Series,
        "tag" => Tag,
        "domain" => Domain,
        "file_hash" => FileHash,
        "perceptual_hash" => PerceptualHash,
        // `Query` always evaluates to non-matching (see
        // `BlockRule::evaluate`), the safest fallback for a rule type
        // this build doesn't recognize.
        _ => Query,
    }
}

fn scope_from_str(s: &str) -> domain::Scope {
    use domain::Scope::*;
    match s {
        "local" => Local,
        "external" => External,
        "selected_sources" => SelectedSources,
        _ => All,
    }
}

fn state_to_str(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Queued => "queued",
        DownloadState::Active => "active",
        DownloadState::Paused => "paused",
        DownloadState::Completed => "completed",
        DownloadState::Failed => "failed",
        DownloadState::Canceled => "canceled",
        DownloadState::Evicted => "evicted",
    }
}

fn state_from_str(s: &str) -> DownloadState {
    match s {
        "active" => DownloadState::Active,
        "paused" => DownloadState::Paused,
        "completed" => DownloadState::Completed,
        "failed" => DownloadState::Failed,
        "canceled" => DownloadState::Canceled,
        "evicted" => DownloadState::Evicted,
        _ => DownloadState::Queued,
    }
}

fn checksum_state_to_str(state: ChecksumState) -> &'static str {
    match state {
        ChecksumState::Pending => "pending",
        ChecksumState::Verified => "verified",
        ChecksumState::Mismatch => "mismatch",
        ChecksumState::Unavailable => "unavailable",
    }
}

fn checksum_state_from_str(s: &str) -> ChecksumState {
    match s {
        "verified" => ChecksumState::Verified,
        "mismatch" => ChecksumState::Mismatch,
        "unavailable" => ChecksumState::Unavailable,
        _ => ChecksumState::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors::FEED_CONNECTOR_ID;

    fn test_ctx() -> (AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        (ctx, dir)
    }

    /// Adds a feed source and imports one download-eligible remote
    /// item through it — the same path `browse`/`import_remote_item`
    /// exercise in `source.rs`'s own tests, reused here since
    /// eligibility/`add` depend on a real imported variant.
    fn imported_downloadable_item(ctx: &AppContext) -> (ItemId, VariantId) {
        let source = SourceService::add(
            ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();
        let remote_item = domain::RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Episode One".to_string(),
            description: None,
            canonical_url: Some("https://example.test/episode-one".to_string()),
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: Some("https://example.test/files/episode-one.mp3".to_string()),
            download_mime_type: Some("audio/mpeg".to_string()),
            download_size_bytes: Some(1024),
        };
        let item_id = SourceService::import_remote_item(ctx, source.id, remote_item).unwrap();
        let variant_id = {
            let conn = ctx.db.connection();
            let id: String = conn
                .query_row(
                    "SELECT id FROM media_variants WHERE item_id = ?1",
                    params![item_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            VariantId(uuid::Uuid::parse_str(&id).unwrap())
        };
        (item_id, variant_id)
    }

    #[test]
    fn eligibility_is_true_for_a_freshly_imported_downloadable_item() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(report.eligible, "reasons: {:?}", report.reasons);
    }

    #[test]
    fn eligibility_fails_for_an_unknown_variant() {
        let (ctx, _dir) = test_ctx();
        let item_id = ItemId::new();
        let report = DownloadService::check_eligibility(&ctx, item_id, VariantId::new()).unwrap();
        assert!(!report.eligible);
    }

    #[test]
    fn eligibility_fails_when_not_download_permitted() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET download_permitted = 0 WHERE id = ?1",
                params![variant_id.to_string()],
            )
            .unwrap();
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("downloadable")));
    }

    #[test]
    fn eligibility_fails_when_already_local() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET local_path = '/already/here.mp3' WHERE id = ?1",
                params![variant_id.to_string()],
            )
            .unwrap();
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("already local")));
    }

    #[test]
    fn eligibility_fails_when_the_source_is_disabled() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let source_id: String = ctx
            .db
            .connection()
            .query_row(
                "SELECT source_id FROM source_references sr \
                 JOIN media_variants mv ON mv.source_ref_id = sr.id WHERE mv.id = ?1",
                params![variant_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE sources SET enabled = 0 WHERE id = ?1",
                params![source_id],
            )
            .unwrap();
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("disabled")));
    }

    #[test]
    fn eligibility_fails_when_blocked_by_an_exact_item_rule() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        ctx.db
            .connection()
            .execute(
                "INSERT INTO block_rules (id, rule_type, target, scope, created_at, enabled) \
                 VALUES (?1, 'exact_item', ?2, 'all', datetime('now'), 1)",
                params![domain::BlockRuleId::new().to_string(), item_id.to_string()],
            )
            .unwrap();
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("blocked")));
    }

    #[test]
    fn eligibility_fails_when_the_download_would_exceed_the_quota() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        SettingsService::set_download_quota_bytes(&ctx, Some(10)).unwrap();
        let report = DownloadService::check_eligibility(&ctx, item_id, variant_id).unwrap();
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("quota")));
    }

    #[test]
    fn add_rejects_an_ineligible_download() {
        let (ctx, _dir) = test_ctx();
        let item_id = ItemId::new();
        let err = DownloadService::add(&ctx, item_id, VariantId::new()).unwrap_err();
        assert!(matches!(err, AppError::UnsupportedCapability(_)));
    }

    #[test]
    fn add_queues_a_download_with_a_rendered_destination() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
        assert_eq!(download.state, DownloadState::Queued);
        assert_eq!(download.bytes_received, 0);
        // mime_guess's reverse lookup for "audio/mpeg" doesn't
        // guarantee "mp3" specifically among its several valid
        // extensions (mp3, mp2, m2a, mpga, ...) — just that some
        // audio/mpeg extension was picked, not the URL's literal path.
        assert!(
            !download.destination.ends_with(".bin"),
            "{}",
            download.destination
        );
        assert!(
            download.destination.contains("Episode One"),
            "{}",
            download.destination
        );
        assert!(
            download.destination.contains("My Feed"),
            "{}",
            download.destination
        );
    }

    #[test]
    fn list_and_find_round_trip() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

        let found = DownloadService::find(&ctx, download.id).unwrap().unwrap();
        assert_eq!(found.id, download.id);

        let list = DownloadService::list(&ctx, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].item_title, "Episode One");
        assert_eq!(list[0].source_display_name.as_deref(), Some("My Feed"));

        let filtered = DownloadService::list(&ctx, Some(item_id)).unwrap();
        assert_eq!(filtered.len(), 1);
        let empty = DownloadService::list(&ctx, Some(ItemId::new())).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn pause_on_an_unknown_id_is_not_found() {
        let (ctx, _dir) = test_ctx();
        let err = DownloadService::pause(&ctx, DownloadId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn pause_transitions_queued_to_paused() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
        DownloadService::pause(&ctx, download.id).unwrap();
        let found = DownloadService::find(&ctx, download.id).unwrap().unwrap();
        assert_eq!(found.state, DownloadState::Paused);
    }

    #[test]
    fn cancel_removes_the_temp_file_and_marks_canceled() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

        let temp_dir = ctx.data_dir.join("temp").join("downloads");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_path = temp_dir.join(format!("{}.part", download.id));
        fs::write(&temp_path, b"partial").unwrap();
        DownloadService::set_temp_path(&ctx, download.id, Some(&temp_path.to_string_lossy()))
            .unwrap();

        DownloadService::cancel(&ctx, download.id).unwrap();
        let found = DownloadService::find(&ctx, download.id).unwrap().unwrap();
        assert_eq!(found.state, DownloadState::Canceled);
        assert!(!temp_path.exists());
    }

    #[test]
    fn set_pinned_round_trips() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
        assert!(
            !DownloadService::find(&ctx, download.id)
                .unwrap()
                .unwrap()
                .pinned
        );
        DownloadService::set_pinned(&ctx, download.id, true).unwrap();
        assert!(
            DownloadService::find(&ctx, download.id)
                .unwrap()
                .unwrap()
                .pinned
        );
    }

    #[test]
    fn remove_without_delete_file_preserves_a_completed_files_variant_row() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

        let final_path = PathBuf::from(&download.destination);
        fs::create_dir_all(final_path.parent().unwrap()).unwrap();
        fs::write(&final_path, b"downloaded bytes").unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'completed' WHERE id = ?1",
                params![download.id.to_string()],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET local_path = ?2 WHERE id = ?1",
                params![variant_id.to_string(), download.destination],
            )
            .unwrap();

        DownloadService::remove(&ctx, download.id, false).unwrap();
        assert!(DownloadService::find(&ctx, download.id).unwrap().is_none());
        assert!(
            final_path.exists(),
            "file must survive when delete_file is false"
        );
        let local_path: Option<String> = ctx
            .db
            .connection()
            .query_row(
                "SELECT local_path FROM media_variants WHERE id = ?1",
                params![variant_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(local_path.is_some(), "the library reference must survive");
    }

    #[test]
    fn remove_with_delete_file_deletes_a_completed_download() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();

        let final_path = PathBuf::from(&download.destination);
        fs::create_dir_all(final_path.parent().unwrap()).unwrap();
        fs::write(&final_path, b"downloaded bytes").unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'completed' WHERE id = ?1",
                params![download.id.to_string()],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET local_path = ?2 WHERE id = ?1",
                params![variant_id.to_string(), download.destination],
            )
            .unwrap();

        DownloadService::remove(&ctx, download.id, true).unwrap();
        assert!(!final_path.exists());
        let local_path: Option<String> = ctx
            .db
            .connection()
            .query_row(
                "SELECT local_path FROM media_variants WHERE id = ?1",
                params![variant_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(local_path.is_none());
    }

    #[test]
    fn sanitize_path_component_strips_separators_and_traversal_stays_a_single_component() {
        assert_eq!(sanitize_path_component("a/b\\c"), "a_b_c");
        assert_eq!(
            sanitize_path_component("../../etc/passwd"),
            ".._.._etc_passwd"
        );
        assert!(!sanitize_path_component("../../etc/passwd").contains('/'));
        assert_eq!(sanitize_path_component(""), "untitled");
        assert_eq!(sanitize_path_component("trailing.dot. "), "trailing.dot");
    }

    #[test]
    fn render_naming_template_substitutes_tokens_and_preserves_directory_structure() {
        let path = render_naming_template(
            "{source}/{title} [{item_id}]/{sequence}.{ext}",
            "My Title",
            "My Source",
            ItemId::new(),
            Some(SourceId::new()),
            "mp3",
            1,
        );
        let rendered = path.to_string_lossy();
        assert!(rendered.starts_with("My Source/My Title ["));
        assert!(rendered.ends_with("/1.mp3"));
    }

    #[test]
    fn unique_destination_appends_a_suffix_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.mp3");
        fs::write(&path, b"x").unwrap();
        let unique = unique_destination(&path);
        assert_eq!(unique, dir.path().join("a-2.mp3"));
    }

    #[test]
    fn sweep_orphaned_temp_files_removes_files_with_no_matching_row() {
        let (ctx, _dir) = test_ctx();
        let temp_dir = ctx.data_dir.join("temp").join("downloads");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("orphan.part"), b"x").unwrap();

        let (item_id, variant_id) = imported_downloadable_item(&ctx);
        let download = DownloadService::add(&ctx, item_id, variant_id).unwrap();
        fs::write(temp_dir.join(format!("{}.part", download.id)), b"x").unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE downloads SET state = 'paused' WHERE id = ?1",
                params![download.id.to_string()],
            )
            .unwrap();

        let swept = DownloadService::sweep_orphaned_temp_files(&ctx).unwrap();
        assert_eq!(swept, 1);
        assert!(!temp_dir.join("orphan.part").exists());
        assert!(temp_dir.join(format!("{}.part", download.id)).exists());
    }
}
