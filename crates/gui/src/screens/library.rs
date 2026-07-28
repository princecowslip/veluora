//! Library screen: a filterable, searchable list of scanned items,
//! backed by `SearchService`. A vertical list rather than a strict
//! image grid — functional-first fidelity per the approved plan; a
//! pixel-accurate card grid (per docs/52-sample-ui-spec.md) is a
//! follow-up polish pass.

use std::path::PathBuf;
use std::sync::Arc;

use application::{AppContext, ItemService, SearchService, ThumbnailService};
use domain::{ItemId, MediaType};
use iced::widget::{button, column, container, image, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

pub struct Entry {
    pub item_id: ItemId,
    pub title: String,
    pub media_type: MediaType,
    pub favorite: bool,
    pub thumbnail_path: Option<PathBuf>,
}

#[derive(Default)]
pub struct State {
    pub search_input: String,
    pub type_filter: Option<MediaType>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SearchSubmit,
    FilterChanged(Option<MediaType>),
    OpenItem(ItemId),
}

pub enum Effect {
    OpenItem(ItemId),
}

/// Sets an initial search query (e.g. handed off from Home) and runs it.
pub fn set_query(state: &mut State, ctx: &Arc<AppContext>, query: String) {
    state.search_input = query;
    run_search(state, ctx);
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    run_search(state, ctx);
}

fn run_search(state: &mut State, ctx: &Arc<AppContext>) {
    let mut query = state.search_input.clone();
    if let Some(media_type) = state.type_filter {
        let type_str = application::media_classification::media_type_to_str(media_type);
        query = if query.trim().is_empty() {
            format!("type:{type_str}")
        } else {
            format!("{query} type:{type_str}")
        };
    }
    let hits = SearchService::search(ctx, &query, 200, 0)
        .map(|r| r.items)
        .unwrap_or_default();

    state.entries = hits
        .into_iter()
        .filter_map(|hit| {
            let uuid = uuid::Uuid::parse_str(&hit.item_id).ok()?;
            let item_id = ItemId(uuid);
            let thumbnail_path = ItemService::get(ctx, item_id).ok().and_then(|detail| {
                let variant_id = uuid::Uuid::parse_str(&detail.variants.first()?.id).ok()?;
                let path = ThumbnailService::cache_path(ctx, domain::VariantId(variant_id));
                path.exists().then_some(path)
            });
            Some(Entry {
                item_id,
                title: hit.title,
                media_type: hit.media_type,
                favorite: hit.favorite,
                thumbnail_path,
            })
        })
        .collect();
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    message: Message,
) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::SearchChanged(value) => {
            state.search_input = value;
            (Task::none(), None)
        }
        Message::SearchSubmit => {
            run_search(state, ctx);
            (Task::none(), None)
        }
        Message::FilterChanged(filter) => {
            state.type_filter = filter;
            run_search(state, ctx);
            (Task::none(), None)
        }
        Message::OpenItem(item_id) => (Task::none(), Some(Effect::OpenItem(item_id))),
    }
}

const FILTERS: &[(&str, Option<MediaType>)] = &[
    ("All", None),
    ("Video", Some(MediaType::Video)),
    ("Image", Some(MediaType::Image)),
    ("Audio", Some(MediaType::Audio)),
    ("Story", Some(MediaType::Story)),
    ("Comic", Some(MediaType::Comic)),
    ("Manga", Some(MediaType::Manga)),
];

pub fn view(state: &State) -> Element<'_, Message> {
    let search = row![
        text_input("Search...", &state.search_input)
            .on_input(Message::SearchChanged)
            .on_submit(Message::SearchSubmit),
        button("Search").on_press(Message::SearchSubmit),
    ]
    .spacing(8);

    let mut filters = row![].spacing(8);
    for (label, filter) in FILTERS {
        let is_active = state.type_filter == *filter;
        let label_text = if is_active {
            format!("[{label}]")
        } else {
            (*label).to_string()
        };
        filters = filters.push(button(text(label_text)).on_press(Message::FilterChanged(*filter)));
    }

    let mut list = column![].spacing(4);
    if state.entries.is_empty() {
        list = list.push(text("No items match this search."));
    }
    for entry in &state.entries {
        let mut row_content = row![].spacing(12).align_y(iced::Alignment::Center);
        if let Some(path) = &entry.thumbnail_path {
            row_content = row_content.push(
                image(image::Handle::from_path(path))
                    .width(Length::Fixed(64.0))
                    .height(Length::Fixed(64.0)),
            );
        }
        let favorite_marker = if entry.favorite { "* " } else { "" };
        row_content = row_content.push(
            text(format!(
                "{favorite_marker}{}  ·  {:?}",
                entry.title, entry.media_type
            ))
            .width(Length::Fill),
        );
        list = list.push(
            button(row_content)
                .on_press(Message::OpenItem(entry.item_id))
                .width(Length::Fill),
        );
    }

    container(scrollable(column![search, filters, list].spacing(16)))
        .padding(24)
        .into()
}
