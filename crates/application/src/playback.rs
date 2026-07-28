//! Resolving "what does opening this item mean" per media type, and
//! recording playback/reading progress with the completion-threshold
//! rule from `docs/16-media-handling.md`.
//!
//! [`PlaybackService::resolve_open`] deliberately stops at *resolving* a
//! target — actually spawning an external player is a presentation-layer
//! action (CLI today; a future GUI would embed or launch differently),
//! so that stays out of this crate per ADR-002 in
//! `docs/26-architecture-decisions.md` ("UI-specific shortcuts remain
//! outside the core").

use std::path::Path;

use domain::{ItemId, MediaType, Progress};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::media_classification::media_type_from_str;
use crate::stories::StoryService;
use crate::user_state::UserStateService;

/// "Mark complete using a configurable threshold, such as 90-95%" per
/// `docs/16-media-handling.md`'s video playback rules — applied to any
/// progress whose `normalized()` value is available (time-based media,
/// images).
pub const COMPLETION_THRESHOLD: f32 = 0.9;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpenTarget {
    ExternalPlayer {
        local_path: String,
        mime_type: String,
        resume_position_ms: Option<u64>,
    },
    Direct {
        local_path: String,
        mime_type: String,
    },
    Pages {
        page_count: u32,
        resume_page_index: Option<u32>,
    },
    Story {
        chapter_map: serde_json::Value,
        resume_chapter_index: Option<u32>,
        resume_character_offset: Option<u64>,
    },
}

pub struct PlaybackService;

impl PlaybackService {
    /// Resolves what opening `item_id` means, and touches
    /// `user_state.last_opened_at` as a side effect.
    pub fn resolve_open(ctx: &AppContext, item_id: ItemId) -> Result<OpenTarget> {
        let media_type = fetch_media_type(ctx, item_id)?;
        let user_state = UserStateService::get(ctx, item_id)?;

        let target = match media_type {
            MediaType::Video | MediaType::Audio => {
                let (local_path, mime_type) = resolve_variant(ctx, item_id)?;
                let resume_position_ms = match &user_state.progress {
                    Some(Progress::TimeBased { position_ms, .. }) => Some(*position_ms),
                    _ => None,
                };
                OpenTarget::ExternalPlayer {
                    local_path,
                    mime_type,
                    resume_position_ms,
                }
            }
            MediaType::Comic | MediaType::Manga => {
                let (local_path, _mime_type) = resolve_variant(ctx, item_id)?;
                let pages = media::list_pages(Path::new(&local_path))
                    .map_err(|e| AppError::InvalidPath(format!("could not list pages: {e}")))?;
                let resume_page_index = match &user_state.progress {
                    Some(Progress::Comic { page_index, .. }) => Some(*page_index),
                    _ => None,
                };
                OpenTarget::Pages {
                    page_count: pages.len() as u32,
                    resume_page_index,
                }
            }
            MediaType::Story => {
                let doc = StoryService::get(ctx, item_id)?.ok_or_else(|| {
                    AppError::NotFound(format!("story document for item {item_id}"))
                })?;
                let (resume_chapter_index, resume_character_offset) = match &user_state.progress {
                    Some(Progress::Story {
                        chapter_index,
                        character_offset,
                    }) => (Some(*chapter_index), Some(*character_offset)),
                    _ => (None, None),
                };
                OpenTarget::Story {
                    chapter_map: doc.chapter_map,
                    resume_chapter_index,
                    resume_character_offset,
                }
            }
            // Images, galleries, and anything not yet modeled resolve to
            // a direct local-file open — there's no gallery-child
            // ingestion or richer handling to add yet this milestone.
            MediaType::Image | MediaType::Gallery | MediaType::Other => {
                let (local_path, mime_type) = resolve_variant(ctx, item_id)?;
                OpenTarget::Direct {
                    local_path,
                    mime_type,
                }
            }
        };

        UserStateService::touch_last_opened(ctx, item_id)?;
        Ok(target)
    }

    /// Records a playback/reading position. `completed_override` lets
    /// the caller force completion for progress types
    /// [`Progress::normalized`] can't infer from (comic, story,
    /// gallery); otherwise completion is auto-derived from
    /// `normalized() >= COMPLETION_THRESHOLD`.
    pub fn record_progress(
        ctx: &AppContext,
        item_id: ItemId,
        progress: Progress,
        completed_override: Option<bool>,
    ) -> Result<domain::UserState> {
        let completed = completed_override.unwrap_or_else(|| {
            progress
                .normalized()
                .map(|n| n >= COMPLETION_THRESHOLD)
                .unwrap_or(false)
        });
        UserStateService::set_progress(ctx, item_id, &progress, completed)
    }
}

fn fetch_media_type(ctx: &AppContext, item_id: ItemId) -> Result<MediaType> {
    let conn = ctx.db.connection();
    let media_type_str: String = conn
        .query_row(
            "SELECT media_type FROM media_items WHERE id = ?1",
            params![item_id.to_string()],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("item {item_id}")),
            other => database::DatabaseError::from(other).into(),
        })?;
    Ok(media_type_from_str(&media_type_str).unwrap_or(MediaType::Other))
}

fn resolve_variant(ctx: &AppContext, item_id: ItemId) -> Result<(String, String)> {
    let conn = ctx.db.connection();
    conn.query_row(
        "SELECT local_path, mime_type FROM media_variants WHERE item_id = ?1 AND local_path IS NOT NULL LIMIT 1",
        params![item_id.to_string()],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
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
    use domain::StoryFormat;

    fn insert_item(ctx: &AppContext, media_type: &str) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, ?2, 'Test', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string(), media_type],
            )
            .unwrap();
        item_id
    }

    fn insert_variant(ctx: &AppContext, item_id: ItemId, local_path: &str, mime_type: &str) {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format, local_path, download_permitted, cache_permitted)
                 VALUES (?1, ?2, ?3, 'x', ?4, 1, 1)",
                params![domain::VariantId::new().to_string(), item_id.to_string(), mime_type, local_path],
            )
            .unwrap();
    }

    #[test]
    fn resolve_open_for_video_returns_external_player_target_and_touches_last_opened() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "video");
        insert_variant(&ctx, item_id, "/library/movie.mp4", "video/mp4");

        let target = PlaybackService::resolve_open(&ctx, item_id).unwrap();
        assert_eq!(
            target,
            OpenTarget::ExternalPlayer {
                local_path: "/library/movie.mp4".to_string(),
                mime_type: "video/mp4".to_string(),
                resume_position_ms: None,
            }
        );

        let state = UserStateService::get(&ctx, item_id).unwrap();
        assert!(state.last_opened_at.is_some());
    }

    #[test]
    fn resolve_open_for_video_surfaces_prior_resume_position() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "video");
        insert_variant(&ctx, item_id, "/library/movie.mp4", "video/mp4");
        UserStateService::set_progress(
            &ctx,
            item_id,
            &Progress::TimeBased {
                position_ms: 42_000,
                duration_ms: Some(100_000),
            },
            false,
        )
        .unwrap();

        let target = PlaybackService::resolve_open(&ctx, item_id).unwrap();
        match target {
            OpenTarget::ExternalPlayer {
                resume_position_ms, ..
            } => assert_eq!(resume_position_ms, Some(42_000)),
            other => panic!("expected ExternalPlayer, got {other:?}"),
        }
    }

    #[test]
    fn resolve_open_for_image_returns_direct_target() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "image");
        insert_variant(&ctx, item_id, "/library/photo.png", "image/png");

        let target = PlaybackService::resolve_open(&ctx, item_id).unwrap();
        assert_eq!(
            target,
            OpenTarget::Direct {
                local_path: "/library/photo.png".to_string(),
                mime_type: "image/png".to_string(),
            }
        );
    }

    #[test]
    fn resolve_open_for_story_returns_chapter_map() {
        // StoryService::ensure writes a cache file, so this needs a real
        // filesystem data_dir — open_in_memory()'s `:memory:` placeholder
        // path would create a literal `./:memory:/` directory on disk.
        let data_dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(data_dir.path()).unwrap();
        let item_id = insert_item(&ctx, "story");
        let dir = tempfile::tempdir().unwrap();
        let src_path = dir.path().join("story.md");
        std::fs::write(&src_path, "# One\nHello\n").unwrap();
        StoryService::ensure(&ctx, item_id, StoryFormat::Markdown, &src_path).unwrap();

        let target = PlaybackService::resolve_open(&ctx, item_id).unwrap();
        match target {
            OpenTarget::Story { chapter_map, .. } => {
                assert_eq!(chapter_map.as_array().unwrap().len(), 1);
            }
            other => panic!("expected Story, got {other:?}"),
        }
    }

    #[test]
    fn record_progress_auto_completes_time_based_progress_past_the_threshold() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "video");

        let state = PlaybackService::record_progress(
            &ctx,
            item_id,
            Progress::TimeBased {
                position_ms: 9_500,
                duration_ms: Some(10_000),
            },
            None,
        )
        .unwrap();
        assert!(state.completed);
    }

    #[test]
    fn record_progress_does_not_auto_complete_below_the_threshold() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "video");

        let state = PlaybackService::record_progress(
            &ctx,
            item_id,
            Progress::TimeBased {
                position_ms: 5_000,
                duration_ms: Some(10_000),
            },
            None,
        )
        .unwrap();
        assert!(!state.completed);
    }

    #[test]
    fn record_progress_honors_an_explicit_completed_override_for_comics() {
        let ctx = AppContext::open_in_memory().unwrap();
        let item_id = insert_item(&ctx, "comic");

        let state = PlaybackService::record_progress(
            &ctx,
            item_id,
            Progress::Comic {
                page_index: 3,
                intra_page_position: 0.0,
            },
            Some(true),
        )
        .unwrap();
        assert!(state.completed);
    }
}
