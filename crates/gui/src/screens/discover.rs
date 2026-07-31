//! Discover: unified cross-source search — the local library plus
//! every enabled connector-backed source, in one query. The GUI
//! counterpart of `veloura discover` and `POST /api/v1/discover`,
//! closing the gap `KNOWN_ISSUES.md` flagged for this screen: Sources
//! management (browsing one source at a time) predates this, since
//! Milestone F's connectors shipped after the GUI did.

use std::sync::Arc;

use application::{AppContext, DiscoverHit, DiscoverService, DiscoverSourceStatus, SourceService};
use domain::{ConnectorResult, RemoteItem, SourceId};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Task};

#[derive(Default)]
pub struct State {
    pub query: String,
    pub loading: bool,
    pub hits: Vec<DiscoverHit>,
    pub source_statuses: Vec<DiscoverSourceStatus>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    Submit,
    DiscoverCompleted(Result<application::DiscoverReport, String>),
    Import(RemoteItem, SourceId),
}

/// Discover is search-driven — there's nothing to eagerly load on
/// navigation-in, unlike screens backed by a standing list.
pub fn refresh(_state: &mut State, _ctx: &Arc<AppContext>) {}

pub fn update(state: &mut State, ctx: &Arc<AppContext>, message: Message) -> Task<Message> {
    match message {
        Message::QueryChanged(value) => {
            state.query = value;
        }
        Message::Submit => {
            state.loading = true;
            state.message = None;
            let ctx = ctx.clone();
            let query = state.query.clone();
            return Task::perform(
                async move {
                    DiscoverService::discover(&ctx, &query, None, 25)
                        .await
                        .map_err(|e| e.to_string())
                },
                Message::DiscoverCompleted,
            );
        }
        Message::DiscoverCompleted(result) => {
            state.loading = false;
            match result {
                Ok(report) => {
                    state.hits = report.hits;
                    state.source_statuses = report.sources;
                    state.message = None;
                }
                Err(e) => {
                    state.hits.clear();
                    state.source_statuses.clear();
                    state.message = Some(format!("Could not discover: {e}"));
                }
            }
        }
        Message::Import(item, source_id) => {
            match SourceService::import_remote_item(ctx, source_id, item.clone()) {
                Ok(item_id) => {
                    // Flips the matching row from an Import button to
                    // "Already in library" in place, without
                    // re-running Discover.
                    for hit in &mut state.hits {
                        if hit.source_id == source_id
                            && hit.item.source_item_id == item.source_item_id
                        {
                            hit.local_item_id = Some(item_id);
                        }
                    }
                    state.message = Some(format!("Imported into the library ({item_id})."));
                }
                Err(e) => state.message = Some(format!("Could not import item: {e}")),
            }
        }
    }
    Task::none()
}

fn source_status_line(status: &DiscoverSourceStatus) -> Option<String> {
    let ok = matches!(
        status.status,
        ConnectorResult::Success(_) | ConnectorResult::Partial(_)
    );
    if ok && status.unsupported_clauses.is_empty() {
        return None;
    }
    let mut line = format!("{}: {:?}", status.source_display_name, status.status);
    if !status.unsupported_clauses.is_empty() {
        line.push_str(&format!(
            " (unsupported: {})",
            status.unsupported_clauses.join(", ")
        ));
    }
    Some(line)
}

fn hit_row(hit: &DiscoverHit) -> Element<'_, Message> {
    let mut r = row![
        text(hit.source_display_name.clone()).width(iced::Length::FillPortion(1)),
        text(hit.item.title.clone()).width(iced::Length::FillPortion(2)),
        text(format!("{:?}", hit.item.media_type)),
    ]
    .spacing(8);

    r = if hit.local_item_id.is_some() {
        r.push(text("Already in library"))
    } else {
        r.push(button("Import").on_press(Message::Import(hit.item.clone(), hit.source_id)))
    };

    r.into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![
        text("Discover").size(24),
        row![
            text_input(
                "Search local library and connector sources...",
                &state.query
            )
            .on_input(Message::QueryChanged)
            .on_submit(Message::Submit),
            button("Search").on_press(Message::Submit),
        ]
        .spacing(8),
    ]
    .spacing(12);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    for status in &state.source_statuses {
        if let Some(line) = source_status_line(status) {
            content = content.push(text(line));
        }
    }

    if state.hits.is_empty() {
        content = content.push(text("No results yet."));
    }
    for hit in &state.hits {
        content = content.push(hit_row(hit));
    }

    container(content).padding(24).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors::FEED_CONNECTOR_ID;

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    fn sample_item(source_item_id: &str, title: &str) -> RemoteItem {
        RemoteItem {
            source_item_id: source_item_id.to_string(),
            title: title.to_string(),
            description: None,
            canonical_url: None,
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        }
    }

    #[test]
    fn discover_completed_populates_hits_and_source_statuses() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        let hit = DiscoverHit {
            source_id: SourceId::new(),
            source_display_name: "My Feed".to_string(),
            item: sample_item("guid-1", "An Item"),
            local_item_id: None,
        };

        let _ = update(
            &mut state,
            &ctx,
            Message::DiscoverCompleted(Ok(application::DiscoverReport {
                schema_version: 1,
                query: "hello".to_string(),
                hits: vec![hit],
                sources: Vec::new(),
            })),
        );

        assert_eq!(state.hits.len(), 1);
        assert_eq!(state.hits[0].item.title, "An Item");
        assert!(state.message.is_none());
    }

    #[test]
    fn discover_completed_with_an_error_clears_results_and_sets_a_message() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            hits: vec![DiscoverHit {
                source_id: SourceId::new(),
                source_display_name: "My Feed".to_string(),
                item: sample_item("guid-1", "An Item"),
                local_item_id: None,
            }],
            ..State::default()
        };

        let _ = update(
            &mut state,
            &ctx,
            Message::DiscoverCompleted(Err("network error".to_string())),
        );

        assert!(state.hits.is_empty());
        assert!(state.message.unwrap().contains("network error"));
    }

    #[test]
    fn import_creates_a_library_item_and_flips_the_matching_hit_in_place() {
        let (ctx, _dir) = test_ctx();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();
        let item = sample_item("guid-1", "An Item");
        let mut state = State {
            hits: vec![DiscoverHit {
                source_id: source.id,
                source_display_name: "My Feed".to_string(),
                item: item.clone(),
                local_item_id: None,
            }],
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::Import(item, source.id));

        assert!(state.message.unwrap().contains("Imported"));
        assert!(state.hits[0].local_item_id.is_some());
    }
}
