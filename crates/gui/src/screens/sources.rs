//! Sources: manage connector-backed sources (add, enable/disable,
//! remove, health-check, browse, and import a browsed item into the
//! local library) — the GUI counterpart of `veloura source ...` and
//! `/api/v1/sources`, both of which predate this screen (Milestone F
//! shipped after the GUI did; see `KNOWN_ISSUES.md`).

use std::fmt;
use std::sync::Arc;

use application::{AppContext, SourceService, LOCAL_FILESYSTEM_CONNECTOR_ID};
use connectors::{BOORU_CONNECTOR_ID, FEED_CONNECTOR_ID};
use domain::{ConnectorId, ConnectorResult, HealthState, RemoteItem, Source, SourceId};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Element, Task};

/// Three connectors exist (`LocalFilesystemConnector`, `FeedConnector`,
/// `BooruConnector` — see `application::source`/`connectors::feed`/
/// `connectors::booru`), so a small fixed choice is honest and
/// sufficient; there's no dynamic connector registry to list from yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectorChoice {
    #[default]
    LocalFilesystem,
    Feed,
    Booru,
}

impl ConnectorChoice {
    const ALL: [ConnectorChoice; 3] = [
        ConnectorChoice::LocalFilesystem,
        ConnectorChoice::Feed,
        ConnectorChoice::Booru,
    ];

    fn connector_id(self) -> ConnectorId {
        match self {
            ConnectorChoice::LocalFilesystem => LOCAL_FILESYSTEM_CONNECTOR_ID,
            ConnectorChoice::Feed => FEED_CONNECTOR_ID,
            ConnectorChoice::Booru => BOORU_CONNECTOR_ID,
        }
    }
}

impl fmt::Display for ConnectorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ConnectorChoice::LocalFilesystem => "Local filesystem",
            ConnectorChoice::Feed => "RSS/Atom feed",
            ConnectorChoice::Booru => "Booru (Danbooru/Gelbooru)",
        })
    }
}

/// Which booru API shape a `ConnectorChoice::Booru` source speaks —
/// stored in `configuration_json["flavor"]` as `"danbooru"`/`"gelbooru"`,
/// matching `connectors::booru`'s config parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BooruFlavorChoice {
    #[default]
    Danbooru,
    Gelbooru,
}

impl BooruFlavorChoice {
    const ALL: [BooruFlavorChoice; 2] = [BooruFlavorChoice::Danbooru, BooruFlavorChoice::Gelbooru];

    fn as_config_str(self) -> &'static str {
        match self {
            BooruFlavorChoice::Danbooru => "danbooru",
            BooruFlavorChoice::Gelbooru => "gelbooru",
        }
    }
}

impl fmt::Display for BooruFlavorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BooruFlavorChoice::Danbooru => "Danbooru-compatible",
            BooruFlavorChoice::Gelbooru => "Gelbooru-compatible",
        })
    }
}

fn connector_label(id: ConnectorId) -> &'static str {
    if id == LOCAL_FILESYSTEM_CONNECTOR_ID {
        "Local filesystem"
    } else if id == FEED_CONNECTOR_ID {
        "RSS/Atom feed"
    } else if id == BOORU_CONNECTOR_ID {
        "Booru (Danbooru/Gelbooru)"
    } else {
        "Unknown connector"
    }
}

fn health_label(state: HealthState) -> &'static str {
    match state {
        HealthState::Unknown => "unknown",
        HealthState::Healthy => "healthy",
        HealthState::Degraded => "degraded",
        HealthState::Unreachable => "unreachable",
    }
}

#[derive(Default)]
pub struct State {
    pub sources: Vec<Source>,
    pub message: Option<String>,

    pub adding: bool,
    pub new_connector: ConnectorChoice,
    pub new_display_name: String,
    pub new_feed_url: String,
    pub new_booru_flavor: BooruFlavorChoice,
    pub new_booru_base_url: String,
    pub new_booru_api_key: String,

    /// The source a browse is in flight for, or was last shown for —
    /// needed so an Import button knows which source a `RemoteItem`
    /// came from.
    pub browsing: Option<SourceId>,
    pub browse_results: Vec<RemoteItem>,
}

#[derive(Debug, Clone)]
pub enum Message {
    StartAdding,
    CancelAdding,
    ConnectorChoiceChanged(ConnectorChoice),
    DisplayNameChanged(String),
    FeedUrlChanged(String),
    BooruFlavorChanged(BooruFlavorChoice),
    BooruBaseUrlChanged(String),
    BooruApiKeyChanged(String),
    ConfirmAdd,

    Enable(SourceId),
    Disable(SourceId),
    Remove(SourceId),

    HealthCheck(SourceId),
    HealthCheckCompleted(Result<HealthState, String>),

    Browse(SourceId),
    BrowseCompleted(Result<application::BrowseReport, String>),

    Import(RemoteItem, SourceId),
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.sources = SourceService::list(ctx).unwrap_or_default();
}

pub fn update(state: &mut State, ctx: &Arc<AppContext>, message: Message) -> Task<Message> {
    match message {
        Message::StartAdding => {
            state.adding = true;
            state.new_connector = ConnectorChoice::default();
            state.new_display_name.clear();
            state.new_feed_url.clear();
            state.new_booru_flavor = BooruFlavorChoice::default();
            state.new_booru_base_url.clear();
            state.new_booru_api_key.clear();
        }
        Message::CancelAdding => {
            state.adding = false;
        }
        Message::ConnectorChoiceChanged(choice) => {
            state.new_connector = choice;
        }
        Message::DisplayNameChanged(value) => {
            state.new_display_name = value;
        }
        Message::FeedUrlChanged(value) => {
            state.new_feed_url = value;
        }
        Message::BooruFlavorChanged(flavor) => {
            state.new_booru_flavor = flavor;
        }
        Message::BooruBaseUrlChanged(value) => {
            state.new_booru_base_url = value;
        }
        Message::BooruApiKeyChanged(value) => {
            state.new_booru_api_key = value;
        }
        Message::ConfirmAdd => {
            let connector_id = state.new_connector.connector_id();
            let configuration_json = match state.new_connector {
                ConnectorChoice::LocalFilesystem => serde_json::json!({}),
                ConnectorChoice::Feed => serde_json::json!({ "url": state.new_feed_url.trim() }),
                ConnectorChoice::Booru => {
                    let api_key = state.new_booru_api_key.trim();
                    serde_json::json!({
                        "flavor": state.new_booru_flavor.as_config_str(),
                        "base_url": state.new_booru_base_url.trim(),
                        "api_key": if api_key.is_empty() { None } else { Some(api_key) },
                    })
                }
            };
            match SourceService::add(
                ctx,
                connector_id,
                state.new_display_name.trim().to_string(),
                configuration_json,
            ) {
                Ok(_) => {
                    state.message = Some("Source added.".to_string());
                    state.adding = false;
                }
                Err(e) => state.message = Some(format!("Could not add source: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Enable(id) => {
            match SourceService::set_enabled(ctx, id, true) {
                Ok(()) => state.message = Some("Source enabled.".to_string()),
                Err(e) => state.message = Some(format!("Could not enable source: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Disable(id) => {
            match SourceService::set_enabled(ctx, id, false) {
                Ok(()) => state.message = Some("Source disabled.".to_string()),
                Err(e) => state.message = Some(format!("Could not disable source: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Remove(id) => {
            match SourceService::remove(ctx, id) {
                Ok(()) => state.message = Some("Source removed.".to_string()),
                Err(e) => state.message = Some(format!("Could not remove source: {e}")),
            }
            if state.browsing == Some(id) {
                state.browsing = None;
                state.browse_results.clear();
            }
            refresh(state, ctx);
        }
        Message::HealthCheck(id) => {
            let ctx = ctx.clone();
            return Task::perform(
                async move {
                    SourceService::health_check(&ctx, id)
                        .await
                        .map_err(|e| e.to_string())
                },
                Message::HealthCheckCompleted,
            );
        }
        Message::HealthCheckCompleted(result) => {
            match result {
                Ok(health) => {
                    state.message = Some(format!("Health check: {}.", health_label(health)))
                }
                Err(e) => state.message = Some(format!("Health check failed: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Browse(id) => {
            state.browsing = Some(id);
            state.browse_results.clear();
            let ctx = ctx.clone();
            return Task::perform(
                async move {
                    SourceService::browse(&ctx, id, None)
                        .await
                        .map_err(|e| e.to_string())
                },
                Message::BrowseCompleted,
            );
        }
        Message::BrowseCompleted(result) => match result {
            Ok(report) => match report.result {
                ConnectorResult::Success(items) | ConnectorResult::Partial(items) => {
                    state.message = None;
                    state.browse_results = items;
                }
                other => {
                    state.message = Some(format!("Browse result: {other:?}"));
                    state.browse_results.clear();
                }
            },
            Err(e) => {
                state.message = Some(format!("Could not browse source: {e}"));
                state.browse_results.clear();
            }
        },
        Message::Import(item, source_id) => {
            match SourceService::import_remote_item(ctx, source_id, item) {
                Ok(item_id) => {
                    state.message = Some(format!("Imported into the library ({item_id})."))
                }
                Err(e) => state.message = Some(format!("Could not import item: {e}")),
            }
        }
    }
    Task::none()
}

fn source_row(source: &Source) -> Element<'_, Message> {
    row![
        text(source.display_name.clone()).width(iced::Length::FillPortion(2)),
        text(connector_label(source.connector_id)).width(iced::Length::FillPortion(1)),
        checkbox("Enabled", source.enabled).on_toggle(move |enabled| if enabled {
            Message::Enable(source.id)
        } else {
            Message::Disable(source.id)
        }),
        text(health_label(source.health_state)),
        button("Health check").on_press(Message::HealthCheck(source.id)),
        button("Browse").on_press(Message::Browse(source.id)),
        button("Remove").on_press(Message::Remove(source.id)),
    ]
    .spacing(8)
    .into()
}

fn browse_result_row(item: &RemoteItem, source_id: SourceId) -> Element<'_, Message> {
    row![
        text(item.title.clone()).width(iced::Length::Fill),
        button("Import").on_press(Message::Import(item.clone(), source_id)),
    ]
    .spacing(8)
    .into()
}

fn add_form(state: &State) -> Element<'_, Message> {
    let mut form = column![
        text("Add a source").size(18),
        pick_list(
            ConnectorChoice::ALL,
            Some(state.new_connector),
            Message::ConnectorChoiceChanged,
        ),
        text_input("Display name", &state.new_display_name).on_input(Message::DisplayNameChanged),
    ]
    .spacing(8);

    if state.new_connector == ConnectorChoice::Feed {
        form = form.push(
            text_input("Feed URL (https://...)", &state.new_feed_url)
                .on_input(Message::FeedUrlChanged),
        );
    }

    if state.new_connector == ConnectorChoice::Booru {
        form = form.push(pick_list(
            BooruFlavorChoice::ALL,
            Some(state.new_booru_flavor),
            Message::BooruFlavorChanged,
        ));
        form = form.push(
            text_input("Base URL (https://...)", &state.new_booru_base_url)
                .on_input(Message::BooruBaseUrlChanged),
        );
        form = form.push(
            text_input("API key (optional)", &state.new_booru_api_key)
                .on_input(Message::BooruApiKeyChanged),
        );
    }

    form = form.push(
        row![
            button("Add").on_press(Message::ConfirmAdd),
            button("Cancel").on_press(Message::CancelAdding),
        ]
        .spacing(8),
    );

    form.into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![text("Sources").size(24)].spacing(12);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    if state.adding {
        content = content.push(add_form(state));
    } else {
        content = content.push(button("Add source...").on_press(Message::StartAdding));
    }

    if state.sources.is_empty() {
        content = content.push(text("No sources configured yet."));
    }
    for source in &state.sources {
        content = content.push(source_row(source));
    }

    if let Some(source_id) = state.browsing {
        content = content.push(text("Browse results").size(18));
        if state.browse_results.is_empty() {
            content = content.push(text("No items (or browse is still running)."));
        }
        for item in &state.browse_results {
            content = content.push(browse_result_row(item, source_id));
        }
    }

    container(content).padding(24).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    #[test]
    fn confirm_add_creates_a_local_filesystem_source_by_default() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_display_name: "My Library".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert!(!state.adding);
        assert_eq!(state.sources.len(), 1);
        assert_eq!(state.sources[0].display_name, "My Library");
        assert_eq!(state.sources[0].connector_id, LOCAL_FILESYSTEM_CONNECTOR_ID);
    }

    #[test]
    fn confirm_add_with_the_feed_connector_stores_the_url_in_configuration() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_connector: ConnectorChoice::Feed,
            new_display_name: "My Feed".to_string(),
            new_feed_url: "https://example.test/feed.xml".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert_eq!(state.sources.len(), 1);
        assert_eq!(state.sources[0].connector_id, FEED_CONNECTOR_ID);
        assert_eq!(
            state.sources[0].configuration_json["url"],
            "https://example.test/feed.xml"
        );
    }

    #[test]
    fn confirm_add_with_the_booru_connector_stores_flavor_and_base_url_in_configuration() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_connector: ConnectorChoice::Booru,
            new_display_name: "My Booru".to_string(),
            new_booru_flavor: BooruFlavorChoice::Gelbooru,
            new_booru_base_url: "https://booru.example.test".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert_eq!(state.sources.len(), 1);
        assert_eq!(state.sources[0].connector_id, BOORU_CONNECTOR_ID);
        assert_eq!(state.sources[0].configuration_json["flavor"], "gelbooru");
        assert_eq!(
            state.sources[0].configuration_json["base_url"],
            "https://booru.example.test"
        );
        assert!(state.sources[0].configuration_json["api_key"].is_null());
    }

    #[test]
    fn enable_and_disable_toggle_the_source() {
        let (ctx, _dir) = test_ctx();
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local".to_string(),
            serde_json::json!({}),
        )
        .unwrap();
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(&mut state, &ctx, Message::Disable(source.id));
        assert!(!state.sources[0].enabled);

        let _ = update(&mut state, &ctx, Message::Enable(source.id));
        assert!(state.sources[0].enabled);
    }

    #[test]
    fn remove_deletes_the_source_and_clears_a_matching_browse() {
        let (ctx, _dir) = test_ctx();
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local".to_string(),
            serde_json::json!({}),
        )
        .unwrap();
        let mut state = State {
            browsing: Some(source.id),
            browse_results: vec![RemoteItem {
                source_item_id: "x".to_string(),
                title: "X".to_string(),
                description: None,
                canonical_url: None,
                tags: Vec::new(),
                media_type: domain::MediaType::Other,
                thumbnail_url: None,
                download_url: None,
                download_mime_type: None,
                download_size_bytes: None,
            }],
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::Remove(source.id));

        assert!(state.sources.is_empty());
        assert!(state.browsing.is_none());
        assert!(state.browse_results.is_empty());
    }

    #[test]
    fn health_check_completed_reports_the_result_and_refreshes() {
        let (ctx, _dir) = test_ctx();
        let source = SourceService::add(
            &ctx,
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local".to_string(),
            serde_json::json!({}),
        )
        .unwrap();
        let mut state = State::default();

        let _ = update(
            &mut state,
            &ctx,
            Message::HealthCheckCompleted(Ok(HealthState::Healthy)),
        );

        assert!(state.message.unwrap().contains("healthy"));
        assert_eq!(state.sources.len(), 1);
        let _ = source.id;
    }

    #[test]
    fn browse_completed_with_a_successful_result_populates_browse_results() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        let item = RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "An Item".to_string(),
            description: None,
            canonical_url: None,
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };

        let _ = update(
            &mut state,
            &ctx,
            Message::BrowseCompleted(Ok(application::BrowseReport {
                result: ConnectorResult::Success(vec![item.clone()]),
                unsupported_clauses: Vec::new(),
            })),
        );

        assert_eq!(state.browse_results.len(), 1);
        assert_eq!(state.browse_results[0].title, "An Item");
    }

    #[test]
    fn browse_completed_with_an_error_clears_results_and_sets_a_message() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            browse_results: vec![RemoteItem {
                source_item_id: "x".to_string(),
                title: "X".to_string(),
                description: None,
                canonical_url: None,
                tags: Vec::new(),
                media_type: domain::MediaType::Other,
                thumbnail_url: None,
                download_url: None,
                download_mime_type: None,
                download_size_bytes: None,
            }],
            ..State::default()
        };

        let _ = update(
            &mut state,
            &ctx,
            Message::BrowseCompleted(Err("network error".to_string())),
        );

        assert!(state.browse_results.is_empty());
        assert!(state.message.unwrap().contains("network error"));
    }

    #[test]
    fn import_creates_a_library_item_from_a_browsed_remote_item() {
        let (ctx, _dir) = test_ctx();
        let source = SourceService::add(
            &ctx,
            FEED_CONNECTOR_ID,
            "My Feed".to_string(),
            serde_json::json!({ "url": "https://example.test/feed.xml" }),
        )
        .unwrap();
        let mut state = State::default();
        let item = RemoteItem {
            source_item_id: "guid-1".to_string(),
            title: "Imported Story".to_string(),
            description: None,
            canonical_url: None,
            tags: Vec::new(),
            media_type: domain::MediaType::Story,
            thumbnail_url: None,
            download_url: None,
            download_mime_type: None,
            download_size_bytes: None,
        };

        let _ = update(&mut state, &ctx, Message::Import(item, source.id));

        assert!(state.message.unwrap().contains("Imported"));
    }
}
