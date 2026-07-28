//! Home screen: a "Continue" section for in-progress items, "Recently
//! added" for the newest library additions, and a search box that hands
//! off to the Library screen.

use std::sync::Arc;

use application::{AppContext, SearchHit, SearchService};
use domain::ItemId;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

#[derive(Default)]
pub struct State {
    pub search_input: String,
    pub continue_items: Vec<SearchHit>,
    pub recent_items: Vec<SearchHit>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SearchSubmit,
    OpenItem(ItemId),
}

pub enum Effect {
    SearchLibrary(String),
    OpenItem(ItemId),
}

/// Reloads Home's data from the database — call whenever Home becomes
/// the active screen, since favorites/progress may have changed since
/// last shown.
pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.continue_items = SearchService::continue_items(ctx, 10).unwrap_or_default();
    state.recent_items = SearchService::search(ctx, "", 20, 0)
        .map(|r| r.items)
        .unwrap_or_default();
}

pub fn update(state: &mut State, message: Message) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::SearchChanged(value) => {
            state.search_input = value;
            (Task::none(), None)
        }
        Message::SearchSubmit => {
            let query = state.search_input.clone();
            (Task::none(), Some(Effect::SearchLibrary(query)))
        }
        Message::OpenItem(item_id) => (Task::none(), Some(Effect::OpenItem(item_id))),
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let search = row![
        text_input("Search your library...", &state.search_input)
            .on_input(Message::SearchChanged)
            .on_submit(Message::SearchSubmit),
        button("Search").on_press(Message::SearchSubmit),
    ]
    .spacing(8);

    let content = column![
        search,
        item_section("Continue", &state.continue_items),
        item_section("Recently added", &state.recent_items),
    ]
    .spacing(24);

    container(scrollable(content)).padding(24).into()
}

fn item_section<'a>(title: &'a str, items: &'a [SearchHit]) -> Element<'a, Message> {
    let mut list = column![text(title).size(20)].spacing(8);
    if items.is_empty() {
        list = list.push(text("Nothing here yet."));
    }
    for item in items {
        let Ok(uuid) = uuid::Uuid::parse_str(&item.item_id) else {
            continue;
        };
        list = list.push(
            button(text(format!("{}  ·  {:?}", item.title, item.media_type)))
                .on_press(Message::OpenItem(ItemId(uuid)))
                .width(Length::Fill),
        );
    }
    list.into()
}
