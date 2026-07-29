use domain::{ItemId, Progress, UserState};
use rusqlite::{params, Row};
use time::OffsetDateTime;

use crate::context::AppContext;
use crate::error::Result;
use crate::time_format::{from_rfc3339, to_rfc3339};

pub struct UserStateService;

impl UserStateService {
    /// Returns the item's user state, or `UserState::new(item_id)`
    /// defaults if no row exists yet — favorites/ratings/progress lazily
    /// create the row on first mutation (see `set_favorite`).
    pub fn get(ctx: &AppContext, item_id: ItemId) -> Result<UserState> {
        let conn = ctx.db.connection();
        let result = conn.query_row(
            "SELECT favorite, rating, viewed, completed, progress_json, last_opened_at, queued_at, notes, private_tags, pinned
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

    /// Sets `last_opened_at` to now — called whenever [`crate::playback::PlaybackService::resolve_open`]
    /// resolves an item to open.
    pub fn touch_last_opened(ctx: &AppContext, item_id: ItemId) -> Result<UserState> {
        let now = to_rfc3339(OffsetDateTime::now_utc());
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, last_opened_at) VALUES (?1, ?2)
                 ON CONFLICT(item_id) DO UPDATE SET last_opened_at = excluded.last_opened_at",
                params![item_id.to_string(), now],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }

    /// Persists a playback/reading position and whether the item counts
    /// as completed. Always marks `viewed = true` — recording any
    /// progress implies the item has been opened.
    pub fn set_progress(
        ctx: &AppContext,
        item_id: ItemId,
        progress: &Progress,
        completed: bool,
    ) -> Result<UserState> {
        let progress_json = serde_json::to_string(progress).unwrap_or_else(|_| "null".to_string());
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, progress_json, viewed, completed) VALUES (?1, ?2, 1, ?3)
                 ON CONFLICT(item_id) DO UPDATE SET
                     progress_json = excluded.progress_json,
                     viewed = 1,
                     completed = excluded.completed",
                params![item_id.to_string(), progress_json, completed as i64],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }

    /// Sets (or clears) the item's free-text notes. Stores exactly the
    /// string it's given — encryption-agnostic by design; callers that
    /// want encrypted-at-rest notes run the value through
    /// [`crate::privacy::PrivacyService::encrypt_text`]/`decrypt_text`
    /// on their side of this call.
    pub fn set_notes(ctx: &AppContext, item_id: ItemId, notes: Option<&str>) -> Result<UserState> {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, notes) VALUES (?1, ?2)
                 ON CONFLICT(item_id) DO UPDATE SET notes = excluded.notes",
                params![item_id.to_string(), notes],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }

    /// Sets the item's private tags. Same encryption-agnostic contract
    /// as [`Self::set_notes`].
    pub fn set_private_tags(
        ctx: &AppContext,
        item_id: ItemId,
        tags: &[String],
    ) -> Result<UserState> {
        let json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, private_tags) VALUES (?1, ?2)
                 ON CONFLICT(item_id) DO UPDATE SET private_tags = excluded.private_tags",
                params![item_id.to_string(), json],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }

    /// Clears playback/reading history (progress, viewed, completed,
    /// last-opened, queued) while leaving `favorite`, `rating`, `notes`,
    /// and `private_tags` untouched — docs/17's "clear history but
    /// retain item" deletion mode. A no-op if the item has no
    /// `user_state` row yet.
    pub fn clear_history(ctx: &AppContext, item_id: ItemId) -> Result<UserState> {
        ctx.db
            .connection()
            .execute(
                "UPDATE user_state SET
                     progress_json = NULL,
                     viewed = 0,
                     completed = 0,
                     last_opened_at = NULL,
                     queued_at = NULL
                 WHERE item_id = ?1",
                params![item_id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        Self::get(ctx, item_id)
    }

    /// Sets the cache-eviction-exemption flag — see
    /// `domain::UserState::pinned`'s doc comment.
    pub fn set_pinned(ctx: &AppContext, item_id: ItemId, pinned: bool) -> Result<UserState> {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, pinned) VALUES (?1, ?2)
                 ON CONFLICT(item_id) DO UPDATE SET pinned = excluded.pinned",
                params![item_id.to_string(), pinned as i64],
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
    let pinned: bool = row.get(9)?;

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
        pinned,
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
    fn touch_last_opened_sets_the_timestamp() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let before = UserStateService::get(&ctx, item_id).unwrap();
        assert!(before.last_opened_at.is_none());

        let after = UserStateService::touch_last_opened(&ctx, item_id).unwrap();
        assert!(after.last_opened_at.is_some());
    }

    #[test]
    fn set_progress_persists_progress_marks_viewed_and_sets_completed() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let progress = Progress::TimeBased {
            position_ms: 9_500,
            duration_ms: Some(10_000),
        };
        let state = UserStateService::set_progress(&ctx, item_id, &progress, true).unwrap();
        assert!(state.viewed);
        assert!(state.completed);
        assert_eq!(state.progress, Some(progress));
    }

    #[test]
    fn set_notes_creates_then_updates_then_clears() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let state = UserStateService::set_notes(&ctx, item_id, Some("first note")).unwrap();
        assert_eq!(state.notes.as_deref(), Some("first note"));

        let state = UserStateService::set_notes(&ctx, item_id, Some("updated note")).unwrap();
        assert_eq!(state.notes.as_deref(), Some("updated note"));

        let state = UserStateService::set_notes(&ctx, item_id, None).unwrap();
        assert_eq!(state.notes, None);
    }

    #[test]
    fn set_private_tags_round_trips() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let tags = vec!["a".to_string(), "b".to_string()];
        let state = UserStateService::set_private_tags(&ctx, item_id, &tags).unwrap();
        assert_eq!(state.private_tags, tags);
    }

    #[test]
    fn clear_history_resets_progress_but_keeps_favorite_and_notes() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        UserStateService::set_favorite(&ctx, item_id, true).unwrap();
        UserStateService::set_notes(&ctx, item_id, Some("keep me")).unwrap();
        UserStateService::set_progress(
            &ctx,
            item_id,
            &Progress::TimeBased {
                position_ms: 5_000,
                duration_ms: Some(10_000),
            },
            false,
        )
        .unwrap();
        UserStateService::touch_last_opened(&ctx, item_id).unwrap();

        let state = UserStateService::clear_history(&ctx, item_id).unwrap();
        assert_eq!(state.progress, None);
        assert!(!state.viewed);
        assert!(!state.completed);
        assert_eq!(state.last_opened_at, None);
        assert!(state.favorite, "favorite must survive a history clear");
        assert_eq!(state.notes.as_deref(), Some("keep me"));
    }

    #[test]
    fn clear_history_on_an_item_with_no_state_row_is_a_harmless_no_op() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);
        let state = UserStateService::clear_history(&ctx, item_id).unwrap();
        assert!(!state.favorite);
    }

    #[test]
    fn set_pinned_creates_then_toggles_the_row() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx);

        let state = UserStateService::set_pinned(&ctx, item_id, true).unwrap();
        assert!(state.pinned);

        let state = UserStateService::set_pinned(&ctx, item_id, false).unwrap();
        assert!(!state.pinned);

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
