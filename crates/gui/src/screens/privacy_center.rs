//! Privacy Center: lock status, cache size, telemetry status (always
//! off), and data/cache deletion controls — per `docs/06-ui-ux.md`'s
//! Privacy Center spec. Stored source credentials are honestly reported
//! as empty; no connectors exist yet.

use std::sync::Arc;

use application::{AppContext, PrivacyService, SettingsService};
use iced::widget::{button, column, container, text};
use iced::{Element, Task};

#[derive(Default)]
pub struct State {
    pub cache_size_bytes: u64,
    pub has_password: bool,
    pub last_cleared_at: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ClearCache,
    DeleteAllData,
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.cache_size_bytes = PrivacyService::cache_size_bytes(ctx).unwrap_or(0);
    state.has_password = PrivacyService::has_password(ctx).unwrap_or(false);
    state.last_cleared_at = SettingsService::last_data_cleared_at(ctx).ok().flatten();
}

pub fn update(state: &mut State, ctx: &Arc<AppContext>, message: Message) -> Task<Message> {
    match message {
        Message::ClearCache => match PrivacyService::clear_cache(ctx) {
            Ok(()) => state.message = Some("Cache cleared.".to_string()),
            Err(e) => state.message = Some(format!("Could not clear cache: {e}")),
        },
        Message::DeleteAllData => match PrivacyService::delete_all_local_data(ctx) {
            Ok(()) => state.message = Some("Local data deleted.".to_string()),
            Err(e) => state.message = Some(format!("Could not delete local data: {e}")),
        },
    }
    refresh(state, ctx);
    Task::none()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let lock_status = if state.has_password {
        "A profile password is set; locking and the panic shortcut (Ctrl+Shift+L) are enabled."
    } else {
        "No profile password is set — set one in Settings to enable locking and the panic shortcut."
    };
    let cache_mb = state.cache_size_bytes as f64 / (1024.0 * 1024.0);

    let mut content = column![
        text("Privacy Center").size(24),
        text(lock_status),
        text(format!("Cache size: {cache_mb:.2} MB")),
        button("Clear cache").on_press(Message::ClearCache),
        text("Telemetry: off"),
        text("Stored source credentials: none"),
    ]
    .spacing(12);

    if let Some(cleared) = &state.last_cleared_at {
        content = content.push(text(format!("Last cleared: {cleared}")));
    }
    content = content.push(button("Delete all local data").on_press(Message::DeleteAllData));
    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    container(content).padding(24).into()
}
