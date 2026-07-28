use domain::{ItemId, Progress, UserState};
use rusqlite::{params, Row};

use crate::context::AppContext;
use crate::error::Result;
use crate::time_format::from_rfc3339;

pub struct UserStateService;

impl UserStateService {
    /// Returns the item's user state, or `UserState::new(item_id)`
    /// defaults if no row exists yet — favorites/ratings/progress lazily
    /// create the row on first mutation (see `set_favorite`).
    pub fn get(ctx: &AppContext, item_id: ItemId) -> Result<UserState> {
        let conn = ctx.db.connection();
        let result = conn.query_row(
            "SELECT favorite, rating, viewed, completed, progress_json, last_opened_at, queued_at, notes, private_tags
             FROM user_state WHERE item_id = ?1",
            params![item_id.to_string()],
            |row| row_to_user_state(row, item_id),
        );
        match result {
            Ok(state) => Ok(state),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(UserState::new(item_id)),
            Err(e) => Err(database::DatabaseError::from(e).into()),
        }
    }

    pub fn set_favorite(ctx: &AppContext, item_id: ItemId, favorite: bool) -> Result<UserState> {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, favorite) VALUES (?1, ?2)
                 ON CONFLICT(item_id) DO UPDATE SET favorite = excluded.favorite",
                params![item_id.to_string(), favorite as i64],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }
}

fn row_to_user_state(row: &Row, item_id: ItemId) -> rusqlite::Result<UserState> {
    let favorite: bool = row.get(0)?;
    let rating: Option<i64> = row.get(1)?;
    let viewed: bool = row.get(2)?;
    let completed: bool = row.get(3)?;
    let progress_json: Option<String> = row.get(4)?;
    let last_opened_at: Option<String> = row.get(5)?;
    let queued_at: Option<String> = row.get(6)?;
    let notes: Option<String> = row.get(7)?;
    let private_tags_json: Option<String> = row.get(8)?;

    let progress: Option<Progress> = progress_json.and_then(|s| serde_json::from_str(&s).ok());
    let private_tags: Vec<String> = private_tags_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(UserState {
        item_id,
        favorite,
        rating: rating.map(|r| r as u8),
        viewed,
        completed,
        progress,
        last_opened_at: last_opened_at.as_deref().and_then(from_rfc3339),
        queued_at: queued_at.as_deref().and_then(from_rfc3339),
        notes,
        private_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn get_returns_defaults_for_an_item_with_no_state_row() {
        let ctx = AppContext::open_in_memory().unwrap();
        let state = UserStateService::get(&ctx, ItemId::new()).unwrap();
        assert!(!state.favorite);
        assert_eq!(state.rating, None);
    }

    #[test]
    fn set_favorite_creates_then_updates_the_row() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let state = UserStateService::set_favorite(&ctx, item_id, true).unwrap();
        assert!(state.favorite);

        let state = UserStateService::set_favorite(&ctx, item_id, false).unwrap();
        assert!(!state.favorite);

        let count: i64 = ctx
            .db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM user_state WHERE item_id = ?1",
                params![item_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "must upsert, not insert a second row");
    }

    #[test]
    fn set_favorite_on_a_nonexistent_item_fails() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = UserStateService::set_favorite(&ctx, ItemId::new(), true).unwrap_err();
        assert!(matches!(err, AppError::Database(_)));
    }

    fn insert_item(ctx: &AppContext) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Test', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string()],
            )
            .unwrap();
        item_id
    }
}
