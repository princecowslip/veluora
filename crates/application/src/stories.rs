//! Story ingestion: builds a sanitized [`StoryDocument`] from a
//! plain-text or Markdown source file (via `media::story`) and persists
//! it to the `story_documents` table plus a cache file, per
//! `docs/16-media-handling.md`'s story pipeline.

use std::path::{Path, PathBuf};

use domain::{ItemId, StoryDocument, StoryFormat};
use rusqlite::{params, Row};

use crate::context::AppContext;
use crate::error::{AppError, Result};

pub struct StoryService;

impl StoryService {
    /// Builds the sanitized document from `source_path`, writes it to
    /// the cache, and upserts the `story_documents` row.
    pub fn ensure(
        ctx: &AppContext,
        item_id: ItemId,
        format: StoryFormat,
        source_path: &Path,
    ) -> Result<StoryDocument> {
        let content = media::build_story_document(source_path, format)
            .map_err(|e| AppError::InvalidPath(format!("could not build story document: {e}")))?;

        let cache_path = Self::cache_path(ctx, item_id);
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&cache_path, &content.sanitized_text)?;

        let sanitized_content_location = cache_path.display().to_string();
        let chapter_map_json =
            serde_json::to_string(&content.chapter_map).unwrap_or_else(|_| "[]".to_string());

        ctx.db
            .connection()
            .execute(
                "INSERT INTO story_documents (item_id, format, sanitized_content_location, chapter_map)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(item_id) DO UPDATE SET
                     format = excluded.format,
                     sanitized_content_location = excluded.sanitized_content_location,
                     chapter_map = excluded.chapter_map",
                params![
                    item_id.to_string(),
                    story_format_to_str(format),
                    sanitized_content_location,
                    chapter_map_json,
                ],
            )
            .map_err(database::DatabaseError::from)?;

        Ok(StoryDocument {
            item_id,
            format,
            sanitized_content_location,
            chapter_map: content.chapter_map,
            text_index_location: None,
        })
    }

    pub fn get(ctx: &AppContext, item_id: ItemId) -> Result<Option<StoryDocument>> {
        let conn = ctx.db.connection();
        let result = conn.query_row(
            "SELECT format, sanitized_content_location, chapter_map, text_index_location
             FROM story_documents WHERE item_id = ?1",
            params![item_id.to_string()],
            |row| row_to_story_document(row, item_id),
        );
        match result {
            Ok(doc) => Ok(Some(doc)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(database::DatabaseError::from(e).into()),
        }
    }

    /// Reads the sanitized content back from the cache file.
    pub fn read_content(ctx: &AppContext, item_id: ItemId) -> Result<String> {
        let doc = Self::get(ctx, item_id)?
            .ok_or_else(|| AppError::NotFound(format!("story document for item {item_id}")))?;
        Ok(std::fs::read_to_string(doc.sanitized_content_location)?)
    }

    fn cache_path(ctx: &AppContext, item_id: ItemId) -> PathBuf {
        ctx.data_dir
            .join("cache")
            .join("stories")
            .join(format!("{item_id}.txt"))
    }
}

fn row_to_story_document(row: &Row, item_id: ItemId) -> rusqlite::Result<StoryDocument> {
    let format_str: String = row.get(0)?;
    let sanitized_content_location: String = row.get(1)?;
    let chapter_map_json: String = row.get(2)?;
    let text_index_location: Option<String> = row.get(3)?;

    Ok(StoryDocument {
        item_id,
        format: story_format_from_str(&format_str).unwrap_or(StoryFormat::PlainText),
        sanitized_content_location,
        chapter_map: serde_json::from_str(&chapter_map_json)
            .unwrap_or(serde_json::Value::Array(Vec::new())),
        text_index_location,
    })
}

fn story_format_to_str(format: StoryFormat) -> &'static str {
    match format {
        StoryFormat::PlainText => "plain_text",
        StoryFormat::Markdown => "markdown",
        StoryFormat::Html => "html",
        StoryFormat::Epub => "epub",
    }
}

fn story_format_from_str(s: &str) -> Option<StoryFormat> {
    Some(match s {
        "plain_text" => StoryFormat::PlainText,
        "markdown" => StoryFormat::Markdown,
        "html" => StoryFormat::Html,
        "epub" => StoryFormat::Epub,
        _ => return None,
    })
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
                 VALUES (?1, 'story', 'A Story', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string()],
            )
            .unwrap();
        item_id
    }

    fn test_ctx() -> (AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        (ctx, dir)
    }

    #[test]
    fn ensure_persists_sanitized_content_and_chapter_map() {
        let (ctx, _ctx_dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("story.md");
        std::fs::write(&src_path, "# Intro\nHello <b>world</b>\n").unwrap();

        let doc = StoryService::ensure(&ctx, item_id, StoryFormat::Markdown, &src_path).unwrap();
        assert_eq!(doc.format, StoryFormat::Markdown);
        assert_eq!(doc.chapter_map.as_array().unwrap().len(), 1);

        let fetched = StoryService::get(&ctx, item_id).unwrap().unwrap();
        assert_eq!(
            fetched.sanitized_content_location,
            doc.sanitized_content_location
        );

        let content = StoryService::read_content(&ctx, item_id).unwrap();
        assert!(content.contains("Hello world"));
    }

    #[test]
    fn ensure_is_idempotent_via_upsert() {
        let (ctx, _ctx_dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("story.txt");
        std::fs::write(&src_path, "first version").unwrap();

        StoryService::ensure(&ctx, item_id, StoryFormat::PlainText, &src_path).unwrap();
        std::fs::write(&src_path, "second version").unwrap();
        StoryService::ensure(&ctx, item_id, StoryFormat::PlainText, &src_path).unwrap();

        let count: i64 = ctx
            .db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM story_documents WHERE item_id = ?1",
                params![item_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "must upsert, not insert a second row");

        let content = StoryService::read_content(&ctx, item_id).unwrap();
        assert_eq!(content, "second version");
    }

    #[test]
    fn get_returns_none_for_an_item_with_no_story_document() {
        let (ctx, _ctx_dir) = test_ctx();
        let item_id = insert_item(&ctx);
        assert!(StoryService::get(&ctx, item_id).unwrap().is_none());
    }

    #[test]
    fn read_content_on_missing_document_is_not_found() {
        let (ctx, _ctx_dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let err = StoryService::read_content(&ctx, item_id).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
