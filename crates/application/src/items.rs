use std::fs;

use domain::{ItemId, MediaType, VariantId};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::media_classification::media_type_from_str;
use crate::thumbnails::ThumbnailService;
use crate::user_state::UserStateService;

/// Assembled from targeted queries (base row + variants + tags), not a
/// full `domain::MediaItem` reconstruction — most of that struct's
/// vector fields live in tables nothing but the scanner populates yet.
#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDetail {
    pub id: String,
    pub title: String,
    pub media_type: MediaType,
    pub favorite: bool,
    pub pinned: bool,
    pub rating: Option<u8>,
    pub variants: Vec<VariantSummary>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VariantSummary {
    pub id: String,
    pub local_path: Option<String>,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub download_permitted: bool,
}

/// What [`ItemService::delete`] actually removed — the "deletion
/// verification" Milestone E asks for: a caller (tests, or the GUI) can
/// confirm exactly what happened rather than just trusting `Ok(())`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeletionReport {
    pub db_rows_deleted: u32,
    pub thumbnails_deleted: u32,
    pub local_files_deleted: u32,
}

pub struct ItemService;

impl ItemService {
    pub fn get(ctx: &AppContext, item_id: ItemId) -> Result<ItemDetail> {
        let (title, media_type_str) = {
            let conn = ctx.db.connection();
            conn.query_row(
                "SELECT title, media_type FROM media_items WHERE id = ?1",
                params![item_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("item {item_id}"))
                }
                other => database::DatabaseError::from(other).into(),
            })?
        };
        let media_type = media_type_from_str(&media_type_str).unwrap_or(MediaType::Other);
        let user_state = UserStateService::get(ctx, item_id)?;

        let variants = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare("SELECT id, local_path, mime_type, width, height, download_permitted FROM media_variants WHERE item_id = ?1")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map(params![item_id.to_string()], |row| {
                    Ok(VariantSummary {
                        id: row.get(0)?,
                        local_path: row.get(1)?,
                        mime_type: row.get(2)?,
                        width: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                        height: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                        download_permitted: row.get::<_, i64>(5)? != 0,
                    })
                })
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(database::DatabaseError::from)?);
            }
            out
        };

        let tags = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare(
                    "SELECT t.display_value FROM media_item_tags mit
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
            out
        };

        Ok(ItemDetail {
            id: item_id.to_string(),
            title,
            media_type,
            favorite: user_state.favorite,
            pinned: user_state.pinned,
            rating: user_state.rating,
            variants,
            tags,
        })
    }

    /// Permanently deletes an item: its variants, tags, collection
    /// membership, user state, and story-document rows, plus every
    /// cached thumbnail and — if `delete_local_files` is set — the
    /// underlying media files on disk. Deletion of the DB rows happens
    /// inside one transaction (manual cleanup of each child table,
    /// matching `LibraryRootService::remove`'s existing pattern rather
    /// than relying on FK cascade behavior); file deletion happens
    /// first, on a best-effort basis, and is reported but never fatal.
    pub fn delete(
        ctx: &AppContext,
        item_id: ItemId,
        delete_local_files: bool,
    ) -> Result<DeletionReport> {
        if !Self::exists(ctx, item_id)? {
            return Err(AppError::NotFound(format!("item {item_id}")));
        }

        let variants: Vec<(VariantId, Option<String>)> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare("SELECT id, local_path FROM media_variants WHERE item_id = ?1")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map(params![item_id.to_string()], |row| {
                    let id_str: String = row.get(0)?;
                    let local_path: Option<String> = row.get(1)?;
                    Ok((id_str, local_path))
                })
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in rows {
                let (id_str, local_path) = row.map_err(database::DatabaseError::from)?;
                let variant_id = VariantId(uuid::Uuid::parse_str(&id_str).unwrap_or_default());
                out.push((variant_id, local_path));
            }
            out
        };

        let mut report = DeletionReport::default();
        for (variant_id, local_path) in &variants {
            let thumb_path = ThumbnailService::cache_path(ctx, *variant_id);
            if thumb_path.exists() && fs::remove_file(&thumb_path).is_ok() {
                report.thumbnails_deleted += 1;
            }
            if delete_local_files {
                if let Some(path) = local_path {
                    if fs::remove_file(path).is_ok() {
                        report.local_files_deleted += 1;
                    }
                }
            }
        }

        let item_id_str = item_id.to_string();
        let mut conn = ctx.db.connection();
        let tx = conn.transaction().map_err(database::DatabaseError::from)?;
        let mut rows_deleted = 0u32;
        for sql in [
            "DELETE FROM media_item_tags WHERE item_id = ?1",
            "DELETE FROM collection_items WHERE item_id = ?1",
            "DELETE FROM user_state WHERE item_id = ?1",
            "DELETE FROM story_documents WHERE item_id = ?1",
            "DELETE FROM media_variants WHERE item_id = ?1",
        ] {
            rows_deleted += tx
                .execute(sql, params![item_id_str])
                .map_err(database::DatabaseError::from)? as u32;
        }
        rows_deleted += tx
            .execute(
                "DELETE FROM media_items WHERE id = ?1",
                params![item_id_str],
            )
            .map_err(database::DatabaseError::from)? as u32;
        tx.commit().map_err(database::DatabaseError::from)?;
        report.db_rows_deleted = rows_deleted;

        Ok(report)
    }

    fn exists(ctx: &AppContext, item_id: ItemId) -> Result<bool> {
        let conn = ctx.db.connection();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE id = ?1",
                params![item_id.to_string()],
                |r| r.get(0),
            )
            .map_err(database::DatabaseError::from)?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_item(ctx: &AppContext) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'A Photo', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string()],
            )
            .unwrap();
        item_id
    }

    #[test]
    fn get_returns_not_found_for_an_unknown_item() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = ItemService::get(&ctx, ItemId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn get_assembles_title_favorite_variants_and_tags() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, width, height, download_permitted, cache_permitted)
                 VALUES ('v1', ?1, 'image/png', 'png', '/x/photo.png', 800, 600, 1, 1)",
                params![item_id.to_string()],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO tags (id, namespace, normalized_value, display_value) VALUES ('t1', 'user', 'nice', 'Nice')",
                [],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_item_tags (item_id, tag_id) VALUES (?1, 't1')",
                params![item_id.to_string()],
            )
            .unwrap();
        UserStateService::set_favorite(&ctx, item_id, true).unwrap();

        let detail = ItemService::get(&ctx, item_id).unwrap();
        assert_eq!(detail.title, "A Photo");
        assert_eq!(detail.media_type, MediaType::Image);
        assert!(detail.favorite);
        assert_eq!(detail.variants.len(), 1);
        assert_eq!(
            detail.variants[0].local_path.as_deref(),
            Some("/x/photo.png")
        );
        assert_eq!(detail.variants[0].width, Some(800));
        assert_eq!(detail.tags, vec!["Nice".to_string()]);
    }

    /// A real filesystem `data_dir` — needed because deletion also
    /// touches thumbnail cache files on disk, unlike most other tests
    /// in this crate which are fine with `AppContext::open_in_memory()`.
    fn test_ctx() -> (AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        (ctx, dir)
    }

    #[test]
    fn delete_removes_db_rows_and_files_when_requested() {
        let (ctx, _ctx_dir) = test_ctx();
        let media_dir = tempfile::tempdir().unwrap();
        let file_path = media_dir.path().join("photo.png");
        std::fs::write(&file_path, b"fake png bytes").unwrap();

        let item_id = insert_item(&ctx);
        let variant_id = domain::VariantId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, download_permitted, cache_permitted)
                 VALUES (?1, ?2, 'image/png', 'png', ?3, 1, 1)",
                params![
                    variant_id.to_string(),
                    item_id.to_string(),
                    file_path.to_str().unwrap()
                ],
            )
            .unwrap();

        let thumb_path = ThumbnailService::cache_path(&ctx, variant_id);
        std::fs::create_dir_all(thumb_path.parent().unwrap()).unwrap();
        std::fs::write(&thumb_path, b"thumb bytes").unwrap();

        UserStateService::set_favorite(&ctx, item_id, true).unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO tags (id, namespace, normalized_value, display_value) VALUES ('t1', 'user', 'x', 'X')",
                [],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_item_tags (item_id, tag_id) VALUES (?1, 't1')",
                params![item_id.to_string()],
            )
            .unwrap();

        let report = ItemService::delete(&ctx, item_id, true).unwrap();
        assert_eq!(report.thumbnails_deleted, 1);
        assert_eq!(report.local_files_deleted, 1);
        assert!(
            report.db_rows_deleted >= 4,
            "tags + user_state + variant + item rows"
        );

        assert!(!thumb_path.exists(), "thumbnail file must be deleted");
        assert!(
            !file_path.exists(),
            "local file must be deleted when requested"
        );

        let item_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(item_count, 0);
        let variant_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_variants", [], |r| r.get(0))
            .unwrap();
        assert_eq!(variant_count, 0);
        let tag_link_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_item_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tag_link_count, 0);
    }

    #[test]
    fn delete_without_local_files_keeps_the_file_on_disk() {
        let (ctx, _ctx_dir) = test_ctx();
        let media_dir = tempfile::tempdir().unwrap();
        let file_path = media_dir.path().join("photo.png");
        std::fs::write(&file_path, b"fake png bytes").unwrap();

        let item_id = insert_item(&ctx);
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, download_permitted, cache_permitted)
                 VALUES ('v1', ?1, 'image/png', 'png', ?2, 1, 1)",
                params![item_id.to_string(), file_path.to_str().unwrap()],
            )
            .unwrap();

        let report = ItemService::delete(&ctx, item_id, false).unwrap();
        assert_eq!(report.local_files_deleted, 0);
        assert!(
            file_path.exists(),
            "the file must survive when delete_local_files is false"
        );
    }

    #[test]
    fn delete_an_unknown_item_is_not_found() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = ItemService::delete(&ctx, ItemId::new(), false).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
