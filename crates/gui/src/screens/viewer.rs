//! Viewer screen: resolves an item via `PlaybackService::resolve_open`
//! and branches on `OpenTarget`. Video/audio launch a configured
//! external player rather than embedding playback (matches what
//! Milestone C's CLI already does — embedding a video decoder in iced
//! is out of scope for a functional-first MVP).

use std::sync::Arc;

use application::{
    AppContext, ComicService, ItemService, OpenTarget, PlaybackService, SettingsService,
    StoryService,
};
use domain::{ItemId, Progress};
use iced::widget::{button, column, container, image, row, scrollable, text};
use iced::{Element, Length, Task};

#[derive(Default)]
pub struct State {
    pub item_id: Option<ItemId>,
    pub item_title: String,
    pub target: Option<OpenTarget>,
    pub comic_page_index: u32,
    pub comic_page_bytes: Option<Vec<u8>>,
    pub story_content: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenExternally,
    MarkStarted,
    MarkWatched,
    NextPage,
    PrevPage,
    SelectChapter(u32),
}

/// Resolves and loads `item_id` into the viewer, touching progress where
/// the media type implies an automatic mark (images mark viewed on
/// open; comics/stories mark their initial page/chapter).
pub fn load(state: &mut State, ctx: &Arc<AppContext>, item_id: ItemId) {
    *state = State::default();
    state.item_id = Some(item_id);
    state.item_title = ItemService::get(ctx, item_id)
        .map(|d| d.title)
        .unwrap_or_default();

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

pub fn update(state: &mut State, ctx: &Arc<AppContext>, message: Message) -> Task<Message> {
    let Some(item_id) = state.item_id else {
        return Task::none();
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
            Task::none()
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
            Task::none()
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
            Task::none()
        }
        Message::NextPage => {
            if let Some(OpenTarget::Pages { page_count, .. }) = &state.target {
                if state.comic_page_index + 1 < *page_count {
                    state.comic_page_index += 1;
                    load_comic_page(state, ctx);
                }
            }
            Task::none()
        }
        Message::PrevPage => {
            if state.comic_page_index > 0 {
                state.comic_page_index -= 1;
                load_comic_page(state, ctx);
            }
            Task::none()
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
            Task::none()
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

    container(content).padding(24).into()
}
