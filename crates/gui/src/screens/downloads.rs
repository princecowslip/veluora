//! Downloads and offline use (Workstream 11) — the GUI counterpart of
//! `veloura download ...` and `/api/v1/downloads`. Lists the download
//! queue and exposes pause/resume/cancel/pin/remove; the entry point
//! that actually queues a download lives on the Viewer screen (a
//! "Download" button shown when the open item's variant permits it).

use std::sync::Arc;

use application::{AppContext, DownloadService, DownloadSummary};
use domain::{Download, DownloadId, DownloadState};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Task};
use tokio::sync::Semaphore;

#[derive(Default)]
pub struct State {
    pub downloads: Vec<DownloadSummary>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    Pause(DownloadId),
    Resume(DownloadId),
    RunFinished(Box<Result<Download, String>>),
    Cancel(DownloadId),
    TogglePin(DownloadId, bool),
    Remove(DownloadId, bool),
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    match DownloadService::list(ctx, None) {
        Ok(downloads) => {
            state.downloads = downloads;
        }
        Err(e) => state.message = Some(format!("Could not load downloads: {e}")),
    }
}

/// Whether a live-progress polling subscription should be active — see
/// `App::subscription`.
pub fn has_in_flight_downloads(state: &State) -> bool {
    state.downloads.iter().any(|d| {
        matches!(
            d.download.state,
            DownloadState::Active | DownloadState::Queued
        )
    })
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    download_semaphore: &Arc<Semaphore>,
    message: Message,
) -> Task<Message> {
    match message {
        Message::Refresh => {
            refresh(state, ctx);
            Task::none()
        }
        Message::Pause(id) => {
            if let Err(e) = DownloadService::pause(ctx, id) {
                state.message = Some(format!("Could not pause download: {e}"));
            }
            refresh(state, ctx);
            Task::none()
        }
        Message::Resume(id) => {
            let ctx = ctx.clone();
            let semaphore = download_semaphore.clone();
            Task::perform(
                async move {
                    let _permit = semaphore.acquire_owned().await;
                    Box::new(
                        DownloadService::resume(&ctx, id)
                            .await
                            .map_err(|e| e.to_string()),
                    )
                },
                Message::RunFinished,
            )
        }
        Message::RunFinished(result) => {
            if let Err(e) = *result {
                state.message = Some(format!("Download error: {e}"));
            }
            refresh(state, ctx);
            Task::none()
        }
        Message::Cancel(id) => {
            if let Err(e) = DownloadService::cancel(ctx, id) {
                state.message = Some(format!("Could not cancel download: {e}"));
            }
            refresh(state, ctx);
            Task::none()
        }
        Message::TogglePin(id, pinned) => {
            if let Err(e) = DownloadService::set_pinned(ctx, id, pinned) {
                state.message = Some(format!("Could not update pin: {e}"));
            }
            refresh(state, ctx);
            Task::none()
        }
        Message::Remove(id, delete_file) => {
            if let Err(e) = DownloadService::remove(ctx, id, delete_file) {
                state.message = Some(format!("Could not remove download: {e}"));
            }
            refresh(state, ctx);
            Task::none()
        }
    }
}

fn download_row(summary: &DownloadSummary) -> Element<'_, Message> {
    let d = &summary.download;
    let source = summary.source_display_name.as_deref().unwrap_or("-");
    let pin_mark = if d.pinned { "* " } else { "" };
    let progress = format!(
        "{}/{} bytes",
        d.bytes_received,
        d.bytes_total
            .map(|b| b.to_string())
            .unwrap_or_else(|| "?".to_string())
    );

    let mut r = row![
        text(format!("{pin_mark}{}", summary.item_title)).width(iced::Length::FillPortion(2)),
        text(source).width(iced::Length::FillPortion(1)),
        text(format!("{:?}", d.state)),
        text(progress),
    ]
    .spacing(8);

    r = match d.state {
        DownloadState::Queued | DownloadState::Active => {
            r.push(button("Pause").on_press(Message::Pause(d.id)))
        }
        DownloadState::Paused | DownloadState::Failed => {
            r.push(button("Resume").on_press(Message::Resume(d.id)))
        }
        DownloadState::Completed | DownloadState::Canceled | DownloadState::Evicted => r,
    };
    if !matches!(
        d.state,
        DownloadState::Completed | DownloadState::Canceled | DownloadState::Evicted
    ) {
        r = r.push(button("Cancel").on_press(Message::Cancel(d.id)));
    }
    r = r.push(
        button(if d.pinned { "Unpin" } else { "Pin" })
            .on_press(Message::TogglePin(d.id, !d.pinned)),
    );
    r = r.push(button("Remove").on_press(Message::Remove(d.id, false)));

    r.into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![row![
        text("Downloads").size(24),
        button("Refresh").on_press(Message::Refresh),
    ]
    .spacing(8),]
    .spacing(12);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    if state.downloads.is_empty() {
        content = content.push(text("No downloads yet."));
    }
    for summary in &state.downloads {
        content = content.push(download_row(summary));
    }

    container(content).padding(24).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors::FEED_CONNECTOR_ID;
    use domain::{ItemId, RemoteItem, VariantId};

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    fn test_semaphore() -> Arc<Semaphore> {
        Arc::new(Semaphore::new(10))
    }

    fn queue_a_download(ctx: &Arc<AppContext>) -> (ItemId, VariantId, DownloadId) {
        let source = application::SourceService::add(
            ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();
        let remote_item = RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Episode One".to_string(),
            description: None,
            canonical_url: Some("https://example.test/episode-one".to_string()),
            tags: Vec::new(),
            media_type: domain::MediaType::Video,
            thumbnail_url: None,
            download_url: Some("https://example.test/files/episode-one.mp3".to_string()),
            download_mime_type: Some("audio/mpeg".to_string()),
            download_size_bytes: Some(1024),
        };
        let item_id =
            application::SourceService::import_remote_item(ctx, source.id, remote_item).unwrap();
        let variant_id = {
            let conn = ctx.db.connection();
            let id: String = conn
                .query_row(
                    "SELECT id FROM media_variants WHERE item_id = ?1",
                    rusqlite::params![item_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            VariantId(uuid::Uuid::parse_str(&id).unwrap())
        };
        let download = DownloadService::add(ctx, item_id, variant_id).unwrap();
        (item_id, variant_id, download.id)
    }

    #[test]
    fn refresh_populates_the_download_list() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, _variant_id, download_id) = queue_a_download(&ctx);

        let mut state = State::default();
        refresh(&mut state, &ctx);

        assert_eq!(state.downloads.len(), 1);
        assert_eq!(state.downloads[0].download.id, download_id);
        assert_eq!(state.downloads[0].item_title, "Episode One");
    }

    #[test]
    fn pause_message_pauses_and_refreshes() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, _variant_id, download_id) = queue_a_download(&ctx);
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            Message::Pause(download_id),
        );
        assert_eq!(state.downloads[0].download.state, DownloadState::Paused);
    }

    #[test]
    fn toggle_pin_message_updates_the_row() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, _variant_id, download_id) = queue_a_download(&ctx);
        let mut state = State::default();
        refresh(&mut state, &ctx);
        assert!(!state.downloads[0].download.pinned);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            Message::TogglePin(download_id, true),
        );
        assert!(state.downloads[0].download.pinned);
    }

    #[test]
    fn remove_message_deletes_the_row() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, _variant_id, download_id) = queue_a_download(&ctx);
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            Message::Remove(download_id, false),
        );
        assert!(state.downloads.is_empty());
    }

    #[test]
    fn has_in_flight_downloads_is_true_while_queued() {
        let (ctx, _dir) = test_ctx();
        queue_a_download(&ctx);
        let mut state = State::default();
        refresh(&mut state, &ctx);
        assert!(has_in_flight_downloads(&state));
    }

    #[test]
    fn has_in_flight_downloads_is_false_once_terminal() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, _variant_id, download_id) = queue_a_download(&ctx);
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(
            &mut state,
            &ctx,
            &test_semaphore(),
            Message::Cancel(download_id),
        );
        assert!(!has_in_flight_downloads(&state));
    }
}
