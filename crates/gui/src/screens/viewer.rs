//! Viewer screen: resolves an item via `PlaybackService::resolve_open`
//! and branches on `OpenTarget`. Video/audio launch a configured
//! external player rather than embedding playback (matches what
//! Milestone C's CLI already does — embedding a video decoder in iced
//! is out of scope for a functional-first MVP).

use std::sync::Arc;

use application::{
    AppContext, ComicService, DownloadService, ItemService, OpenTarget, PlaybackService,
    PrivacyService, SettingsService, StoryService, UserStateService,
};
use domain::{ItemId, Progress, VariantId};
use iced::widget::{button, column, container, image, row, scrollable, text, text_input};
use iced::{Element, Length, Task};
use tokio::sync::Semaphore;

#[derive(Default)]
pub struct State {
    pub item_id: Option<ItemId>,
    pub item_title: String,
    pub target: Option<OpenTarget>,
    pub comic_page_index: u32,
    pub comic_page_bytes: Option<Vec<u8>>,
    pub story_content: Option<String>,
    pub notes_input: String,
    pub pinned: bool,
    pub delete_confirm_armed: bool,
    pub error: Option<String>,
    /// The variant `Message::Download` would queue — `None` when
    /// nothing on this item is `download_permitted && !local_path`.
    pub downloadable_variant_id: Option<VariantId>,
    pub download_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenExternally,
    MarkStarted,
    MarkWatched,
    NextPage,
    PrevPage,
    SelectChapter(u32),
    NotesChanged(String),
    SaveNotes,
    TogglePin,
    ClearHistory,
    ArmDelete,
    CancelDelete,
    ConfirmDelete,
    Download,
    DownloadQueued(Result<(), String>),
}

/// Signals the parent screen needs to act — currently just "the item is
/// gone, navigate back to Library."
pub enum Effect {
    Deleted,
}

/// Resolves and loads `item_id` into the viewer, touching progress where
/// the media type implies an automatic mark (images mark viewed on
/// open; comics/stories mark their initial page/chapter). Decrypts
/// notes with `encryption_key` when one is available; otherwise reads
/// them as plain text (see `application::privacy::PrivacyService::decrypt_text`'s
/// pass-through behavior for unmarked plaintext).
pub fn load(
    state: &mut State,
    ctx: &Arc<AppContext>,
    item_id: ItemId,
    encryption_key: Option<&[u8; 32]>,
) {
    *state = State::default();
    state.item_id = Some(item_id);
    let detail = ItemService::get(ctx, item_id).ok();
    state.item_title = detail.as_ref().map(|d| d.title.clone()).unwrap_or_default();
    state.downloadable_variant_id = detail.as_ref().and_then(|d| {
        d.variants
            .iter()
            .find(|v| v.download_permitted && v.local_path.is_none())
            .and_then(|v| uuid::Uuid::parse_str(&v.id).ok())
            .map(VariantId)
    });

    let user_state = UserStateService::get(ctx, item_id).ok();
    if let Some(state_ref) = &user_state {
        state.pinned = state_ref.pinned;
    }
    if let Some(raw_notes) = user_state.and_then(|s| s.notes) {
        state.notes_input = match encryption_key {
            Some(key) => PrivacyService::decrypt_text(key, &raw_notes).unwrap_or(raw_notes),
            None => raw_notes,
        };
    }

    match PlaybackService::resolve_open(ctx, item_id) {
        Ok(target) => {
            if let OpenTarget::Direct { .. } = &target {
                let _ = PlaybackService::record_progress(
                    ctx,
                    item_id,
                    Progress::Image { viewed: true },
                    None,
                );
            }
            if let OpenTarget::Pages {
                resume_page_index, ..
            } = &target
            {
                state.comic_page_index = resume_page_index.unwrap_or(0);
            }
            state.target = Some(target);
        }
        Err(e) => state.error = Some(e.to_string()),
    }

    if matches!(state.target, Some(OpenTarget::Pages { .. })) {
        load_comic_page(state, ctx);
    }
    if matches!(state.target, Some(OpenTarget::Story { .. })) {
        state.story_content = StoryService::read_content(ctx, item_id).ok();
    }
}

fn load_comic_page(state: &mut State, ctx: &Arc<AppContext>) {
    let Some(item_id) = state.item_id else {
        return;
    };
    match ComicService::page_bytes(ctx, item_id, state.comic_page_index) {
        Ok((bytes, _mime)) => state.comic_page_bytes = Some(bytes),
        Err(e) => {
            state.error = Some(e.to_string());
            state.comic_page_bytes = None;
        }
    }
    let _ = PlaybackService::record_progress(
        ctx,
        item_id,
        Progress::Comic {
            page_index: state.comic_page_index,
            intra_page_position: 0.0,
        },
        None,
    );
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    download_semaphore: &Arc<Semaphore>,
    encryption_key: Option<&[u8; 32]>,
    message: Message,
) -> (Task<Message>, Option<Effect>) {
    let Some(item_id) = state.item_id else {
        return (Task::none(), None);
    };
    match message {
        Message::OpenExternally => {
            if let Some(OpenTarget::ExternalPlayer { local_path, .. }) = &state.target {
                match SettingsService::external_player_path(ctx) {
                    Ok(Some(player)) => {
                        let cmd =
                            media::build_command(&player, std::path::Path::new(local_path), None);
                        if let Err(e) = media::launch(&cmd) {
                            state.error = Some(format!("could not launch player: {e}"));
                        }
                    }
                    _ => {
                        state.error = Some(
                            "No external player configured — set one in Settings.".to_string(),
                        );
                    }
                }
            }
            (Task::none(), None)
        }
        Message::MarkStarted => {
            if matches!(state.target, Some(OpenTarget::ExternalPlayer { .. })) {
                let _ = PlaybackService::record_progress(
                    ctx,
                    item_id,
                    Progress::TimeBased {
                        position_ms: 0,
                        duration_ms: None,
                    },
                    Some(false),
                );
            }
            (Task::none(), None)
        }
        Message::MarkWatched => {
            if matches!(state.target, Some(OpenTarget::ExternalPlayer { .. })) {
                let _ = PlaybackService::record_progress(
                    ctx,
                    item_id,
                    Progress::TimeBased {
                        position_ms: 1,
                        duration_ms: Some(1),
                    },
                    Some(true),
                );
            }
            (Task::none(), None)
        }
        Message::NextPage => {
            if let Some(OpenTarget::Pages { page_count, .. }) = &state.target {
                if state.comic_page_index + 1 < *page_count {
                    state.comic_page_index += 1;
                    load_comic_page(state, ctx);
                }
            }
            (Task::none(), None)
        }
        Message::PrevPage => {
            if state.comic_page_index > 0 {
                state.comic_page_index -= 1;
                load_comic_page(state, ctx);
            }
            (Task::none(), None)
        }
        Message::SelectChapter(index) => {
            if let Some(OpenTarget::Story { chapter_map, .. }) = &state.target {
                if let Some(offset) = chapter_map
                    .as_array()
                    .and_then(|arr| arr.get(index as usize))
                    .and_then(|c| c.get("char_offset"))
                    .and_then(|o| o.as_u64())
                {
                    let _ = PlaybackService::record_progress(
                        ctx,
                        item_id,
                        Progress::Story {
                            chapter_index: index,
                            character_offset: offset,
                        },
                        None,
                    );
                }
            }
            (Task::none(), None)
        }
        Message::NotesChanged(value) => {
            state.notes_input = value;
            (Task::none(), None)
        }
        Message::SaveNotes => {
            let stored_value = if state.notes_input.is_empty() {
                None
            } else if let Some(key) = encryption_key {
                // Fall back to storing plaintext on an encryption
                // failure rather than silently losing the note.
                Some(
                    PrivacyService::encrypt_text(key, &state.notes_input)
                        .unwrap_or_else(|_| state.notes_input.clone()),
                )
            } else {
                Some(state.notes_input.clone())
            };
            let _ = UserStateService::set_notes(ctx, item_id, stored_value.as_deref());
            (Task::none(), None)
        }
        Message::TogglePin => {
            let new_pinned = !state.pinned;
            match UserStateService::set_pinned(ctx, item_id, new_pinned) {
                Ok(_) => state.pinned = new_pinned,
                Err(e) => state.error = Some(e.to_string()),
            }
            (Task::none(), None)
        }
        Message::ClearHistory => {
            let _ = UserStateService::clear_history(ctx, item_id);
            (Task::none(), None)
        }
        Message::ArmDelete => {
            state.delete_confirm_armed = true;
            (Task::none(), None)
        }
        Message::CancelDelete => {
            state.delete_confirm_armed = false;
            (Task::none(), None)
        }
        Message::ConfirmDelete => match ItemService::delete(ctx, item_id, false) {
            Ok(_report) => (Task::none(), Some(Effect::Deleted)),
            Err(e) => {
                state.error = Some(e.to_string());
                state.delete_confirm_armed = false;
                (Task::none(), None)
            }
        },
        Message::Download => {
            let Some(variant_id) = state.downloadable_variant_id else {
                return (Task::none(), None);
            };
            match DownloadService::add(ctx, item_id, variant_id) {
                Ok(download) => {
                    state.download_message = Some("Queued — see Downloads.".to_string());
                    let ctx = ctx.clone();
                    let semaphore = download_semaphore.clone();
                    let id = download.id;
                    (
                        Task::perform(
                            async move {
                                let _permit = semaphore.acquire_owned().await;
                                DownloadService::run(&ctx, id)
                                    .await
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            },
                            Message::DownloadQueued,
                        ),
                        None,
                    )
                }
                Err(e) => {
                    state.download_message = Some(format!("Could not queue download: {e}"));
                    (Task::none(), None)
                }
            }
        }
        Message::DownloadQueued(result) => {
            if let Err(e) = result {
                state.download_message = Some(format!("Download error: {e}"));
            }
            (Task::none(), None)
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![text(&state.item_title).size(24)].spacing(16);
    if let Some(err) = &state.error {
        content = content.push(text(err.clone()));
    }

    match &state.target {
        Some(OpenTarget::Direct { local_path, .. }) => {
            content = content.push(image(image::Handle::from_path(local_path)).width(Length::Fill));
        }
        Some(OpenTarget::ExternalPlayer {
            resume_position_ms, ..
        }) => {
            if let Some(position) = resume_position_ms {
                content = content.push(text(format!("Resume at {position} ms")));
            }
            content = content.push(
                row![
                    button("Open externally").on_press(Message::OpenExternally),
                    button("Mark started").on_press(Message::MarkStarted),
                    button("Mark watched").on_press(Message::MarkWatched),
                ]
                .spacing(8),
            );
        }
        Some(OpenTarget::Pages { page_count, .. }) => {
            if let Some(bytes) = &state.comic_page_bytes {
                content = content
                    .push(image(image::Handle::from_bytes(bytes.clone())).width(Length::Fill));
            }
            content = content.push(text(format!(
                "Page {} of {page_count}",
                state.comic_page_index + 1
            )));
            content = content.push(
                row![
                    button("Previous")
                        .on_press_maybe((state.comic_page_index > 0).then_some(Message::PrevPage)),
                    button("Next").on_press_maybe(
                        (state.comic_page_index + 1 < *page_count).then_some(Message::NextPage)
                    ),
                ]
                .spacing(8),
            );
        }
        Some(OpenTarget::Story { chapter_map, .. }) => {
            if let Some(text_content) = &state.story_content {
                content = content
                    .push(scrollable(text(text_content.clone())).height(Length::Fixed(400.0)));
            }
            if let Some(chapters) = chapter_map.as_array() {
                let mut chapter_row = row![].spacing(8);
                for (index, chapter) in chapters.iter().enumerate() {
                    if let Some(title) = chapter.get("title").and_then(|t| t.as_str()) {
                        chapter_row = chapter_row.push(
                            button(text(title.to_string()))
                                .on_press(Message::SelectChapter(index as u32)),
                        );
                    }
                }
                content = content.push(chapter_row);
            }
        }
        None => {}
    }

    if state.downloadable_variant_id.is_some() {
        content = content.push(button("Download").on_press(Message::Download));
    }
    if let Some(message) = &state.download_message {
        content = content.push(text(message.clone()));
    }

    content = content.push(
        column![
            text("Notes").size(16),
            text_input("Add a note...", &state.notes_input)
                .on_input(Message::NotesChanged)
                .on_submit(Message::SaveNotes),
            button("Save note").on_press(Message::SaveNotes),
        ]
        .spacing(8),
    );

    content = content.push(
        row![
            button(if state.pinned { "Unpin" } else { "Pin" }).on_press(Message::TogglePin),
            button("Clear history").on_press(Message::ClearHistory),
            delete_control(state),
        ]
        .spacing(8),
    );

    container(content).padding(24).into()
}

fn delete_control(state: &State) -> Element<'_, Message> {
    if state.delete_confirm_armed {
        row![
            text("Delete this item permanently?"),
            button("Confirm delete").on_press(Message::ConfirmDelete),
            button("Cancel").on_press(Message::CancelDelete),
        ]
        .spacing(8)
        .into()
    } else {
        button("Delete item...").on_press(Message::ArmDelete).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    fn test_semaphore() -> Arc<Semaphore> {
        Arc::new(Semaphore::new(10))
    }

    fn insert_item(ctx: &AppContext) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Test Item', 'unrated', datetime('now'), datetime('now'))",
                rusqlite::params![item_id.to_string()],
            )
            .unwrap();
        item_id
    }

    #[test]
    fn save_notes_persists_plaintext_without_a_key() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let mut state = State {
            item_id: Some(item_id),
            notes_input: "hello".to_string(),
            ..State::default()
        };

        let (_, effect) = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::SaveNotes,
        );
        assert!(effect.is_none());

        let stored = UserStateService::get(&ctx, item_id).unwrap();
        assert_eq!(stored.notes.as_deref(), Some("hello"));
    }

    #[test]
    fn save_notes_encrypts_when_a_key_is_present_and_decrypts_back() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let key = [3u8; 32];
        let mut state = State {
            item_id: Some(item_id),
            notes_input: "secret".to_string(),
            ..State::default()
        };

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            Some(&key),
            Message::SaveNotes,
        );

        let stored = UserStateService::get(&ctx, item_id).unwrap();
        let raw = stored.notes.unwrap();
        assert!(raw.starts_with("enc:v1:"));
        assert_eq!(PrivacyService::decrypt_text(&key, &raw).unwrap(), "secret");
    }

    #[test]
    fn toggle_pin_flips_state_and_persists() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let mut state = State {
            item_id: Some(item_id),
            ..State::default()
        };
        assert!(!state.pinned);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::TogglePin,
        );
        assert!(state.pinned);
        assert!(UserStateService::get(&ctx, item_id).unwrap().pinned);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::TogglePin,
        );
        assert!(!state.pinned);
        assert!(!UserStateService::get(&ctx, item_id).unwrap().pinned);
    }

    #[test]
    fn clear_history_resets_progress_but_keeps_the_item() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        UserStateService::set_progress(&ctx, item_id, &Progress::Image { viewed: true }, true)
            .unwrap();
        let mut state = State {
            item_id: Some(item_id),
            ..State::default()
        };

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::ClearHistory,
        );

        let stored = UserStateService::get(&ctx, item_id).unwrap();
        assert!(!stored.completed);
    }

    #[test]
    fn arm_then_cancel_delete_does_not_delete_anything() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let mut state = State {
            item_id: Some(item_id),
            ..State::default()
        };

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::ArmDelete,
        );
        assert!(state.delete_confirm_armed);
        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::CancelDelete,
        );
        assert!(!state.delete_confirm_armed);

        let count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    fn insert_downloadable_video_item(ctx: &AppContext) -> ItemId {
        let source = application::SourceService::add(
            ctx,
            connectors::FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();
        let remote_item = domain::RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Downloadable Episode".to_string(),
            description: None,
            canonical_url: Some("https://example.test/episode".to_string()),
            tags: Vec::new(),
            media_type: domain::MediaType::Video,
            thumbnail_url: None,
            download_url: Some("https://example.test/files/episode.mp4".to_string()),
            download_mime_type: Some("video/mp4".to_string()),
            download_size_bytes: Some(2048),
        };
        application::SourceService::import_remote_item(ctx, source.id, remote_item).unwrap()
    }

    #[test]
    fn load_populates_the_downloadable_variant_for_an_eligible_item() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_downloadable_video_item(&ctx);
        let mut state = State::default();

        load(&mut state, &ctx, item_id, None);

        assert!(state.downloadable_variant_id.is_some());
    }

    #[test]
    fn load_has_no_downloadable_variant_for_a_local_only_item() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let mut state = State::default();

        load(&mut state, &ctx, item_id, None);

        assert!(state.downloadable_variant_id.is_none());
    }

    #[test]
    fn download_message_queues_the_download_and_dispatches_a_run_task() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_downloadable_video_item(&ctx);
        let mut state = State::default();
        load(&mut state, &ctx, item_id, None);
        let variant_id = state.downloadable_variant_id.unwrap();

        let (_task, effect) = update(&mut state, &ctx, &test_semaphore(), None, Message::Download);
        assert!(effect.is_none());
        assert!(state.download_message.is_some());

        let downloads = application::DownloadService::list(&ctx, Some(item_id)).unwrap();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].download.variant_id, variant_id);
    }

    #[test]
    fn confirm_delete_removes_the_item_and_yields_the_deleted_effect() {
        let (ctx, _dir) = test_ctx();
        let item_id = insert_item(&ctx);
        let mut state = State {
            item_id: Some(item_id),
            ..State::default()
        };

        let (_, effect) = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            None,
            Message::ConfirmDelete,
        );
        assert!(matches!(effect, Some(Effect::Deleted)));

        let count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
