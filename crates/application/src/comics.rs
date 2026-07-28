//! Comic/manga page listing and serving, resolving an item's local CBZ
//! variant and delegating to `media::archive`.

use domain::ItemId;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub index: u32,
    pub size: u64,
}

pub struct ComicService;

impl ComicService {
    /// Lists an item's comic pages, in reading order.
    pub fn pages(ctx: &AppContext, item_id: ItemId) -> Result<Vec<PageSummary>> {
        let archive_path = resolve_local_path(ctx, item_id)?;
        let pages = media::list_pages(std::path::Path::new(&archive_path))
            .map_err(|e| AppError::InvalidPath(format!("could not list pages: {e}")))?;
        Ok(pages
            .into_iter()
            .map(|p| PageSummary {
                index: p.index,
                size: p.size,
            })
            .collect())
    }

    /// Returns one page's raw bytes plus a best-guess MIME type.
    pub fn page_bytes(ctx: &AppContext, item_id: ItemId, index: u32) -> Result<(Vec<u8>, String)> {
        let archive_path = resolve_local_path(ctx, item_id)?;
        let pages = media::list_pages(std::path::Path::new(&archive_path))
            .map_err(|e| AppError::InvalidPath(format!("could not list pages: {e}")))?;
        let page = pages
            .iter()
            .find(|p| p.index == index)
            .ok_or_else(|| AppError::NotFound(format!("page {index} of item {item_id}")))?;
        let mime = mime_guess::from_path(&page.entry_name)
            .first_or_octet_stream()
            .to_string();
        let bytes = media::read_page(std::path::Path::new(&archive_path), index)
            .map_err(|e| AppError::InvalidPath(format!("could not read page {index}: {e}")))?;
        Ok((bytes, mime))
    }
}

fn resolve_local_path(ctx: &AppContext, item_id: ItemId) -> Result<String> {
    let conn = ctx.db.connection();
    conn.query_row(
        "SELECT local_path FROM media_variants WHERE item_id = ?1 AND local_path IS NOT NULL LIMIT 1",
        params![item_id.to_string()],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("no local file for item {item_id}"))
        }
        other => database::DatabaseError::from(other).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn insert_item_with_variant(ctx: &AppContext, local_path: &str) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'comic', 'A Comic', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string()],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, download_permitted, cache_permitted)
                 VALUES (?1, ?2, 'application/vnd.comicbook+zip', 'cbz', ?3, 1, 1)",
                params![domain::VariantId::new().to_string(), item_id.to_string(), local_path],
            )
            .unwrap();
        item_id
    }

    fn write_cbz(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn pages_lists_the_archive_contents_in_order() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cbz_path = dir.path().join("book.cbz");
        write_cbz(&cbz_path, &[("001.jpg", b"b"), ("000.jpg", b"a")]);
        let item_id = insert_item_with_variant(&ctx, cbz_path.to_str().unwrap());

        let pages = ComicService::pages(&ctx, item_id).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].index, 0);
        assert_eq!(pages[1].index, 1);
    }

    #[test]
    fn page_bytes_returns_the_right_content_and_mime() {
        let ctx = AppContext::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cbz_path = dir.path().join("book.cbz");
        write_cbz(&cbz_path, &[("000.jpg", b"jpeg bytes")]);
        let item_id = insert_item_with_variant(&ctx, cbz_path.to_str().unwrap());

        let (bytes, mime) = ComicService::page_bytes(&ctx, item_id, 0).unwrap();
        assert_eq!(bytes, b"jpeg bytes");
        assert_eq!(mime, "image/jpeg");
    }

    #[test]
    fn pages_on_an_item_without_a_local_file_is_not_found() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = ComicService::pages(&ctx, ItemId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
