use domain::{ItemId, MediaType};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::media_classification::media_type_from_str;
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
                .prepare("SELECT id, local_path, mime_type, width, height FROM media_variants WHERE item_id = ?1")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map(params![item_id.to_string()], |row| {
                    Ok(VariantSummary {
                        id: row.get(0)?,
                        local_path: row.get(1)?,
                        mime_type: row.get(2)?,
                        width: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                        height: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
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
            rating: user_state.rating,
            variants,
            tags,
        })
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
}
