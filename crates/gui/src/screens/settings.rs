//! Settings: theme, library folder management (GUI equivalent of the
//! `library` CLI subcommands), external player path, profile password,
//! start-locked toggle, and a diagnostics panel.

use std::path::PathBuf;
use std::sync::Arc;

use application::{
    AppContext, DiagnosticsService, DiagnosticsSummary, LibraryRootService, PrivacyService,
    SettingsService, Theme as AppTheme,
};
use domain::{LibraryRoot, LibraryRootId};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

pub struct State {
    pub theme: AppTheme,
    pub library_roots: Vec<LibraryRoot>,
    pub external_player_input: String,
    pub password_input: String,
    pub has_password: bool,
    pub start_locked: bool,
    pub diagnostics: Option<DiagnosticsSummary>,
    pub message: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: AppTheme::Dark,
            library_roots: Vec::new(),
            external_player_input: String::new(),
            password_input: String::new(),
            has_password: false,
            start_locked: false,
            diagnostics: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ThemeChanged(AppTheme),
    ExternalPlayerChanged(String),
    ExternalPlayerSave,
    AddLibraryRoot,
    FolderPicked(Option<PathBuf>),
    RemoveLibraryRoot(LibraryRootId),
    PasswordInputChanged(String),
    SetPassword,
    RemovePassword,
    ToggleStartLocked(bool),
}

pub enum Effect {
    ThemeChanged(AppTheme),
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.theme = SettingsService::theme(ctx).unwrap_or(AppTheme::Dark);
    state.library_roots = LibraryRootService::list(ctx).unwrap_or_default();
    state.external_player_input = SettingsService::external_player_path(ctx)
        .ok()
        .flatten()
        .unwrap_or_default();
    state.has_password = PrivacyService::has_password(ctx).unwrap_or(false);
    state.start_locked = SettingsService::start_locked(ctx).unwrap_or(false);
    state.diagnostics = DiagnosticsService::summary(ctx).ok();
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    message: Message,
) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::ThemeChanged(theme) => {
            let _ = SettingsService::set_theme(ctx, theme);
            state.theme = theme;
            (Task::none(), Some(Effect::ThemeChanged(theme)))
        }
        Message::ExternalPlayerChanged(value) => {
            state.external_player_input = value;
            (Task::none(), None)
        }
        Message::ExternalPlayerSave => {
            let _ = SettingsService::set_external_player_path(ctx, &state.external_player_input);
            state.message = Some("Saved.".to_string());
            (Task::none(), None)
        }
        Message::AddLibraryRoot => (
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::FolderPicked,
            ),
            None,
        ),
        Message::FolderPicked(path) => {
            if let Some(path) = path {
                let _ = LibraryRootService::add(ctx, &path, None);
                refresh(state, ctx);
            }
            (Task::none(), None)
        }
        Message::RemoveLibraryRoot(id) => {
            let _ = LibraryRootService::remove(ctx, id);
            refresh(state, ctx);
            (Task::none(), None)
        }
        Message::PasswordInputChanged(value) => {
            state.password_input = value;
            (Task::none(), None)
        }
        Message::SetPassword => {
            if !state.password_input.is_empty() {
                let _ = PrivacyService::set_password(ctx, &state.password_input);
                state.password_input.clear();
                state.has_password = true;
            }
            (Task::none(), None)
        }
        Message::RemovePassword => {
            let _ = PrivacyService::remove_password(ctx);
            let _ = SettingsService::set_start_locked(ctx, false);
            state.has_password = false;
            state.start_locked = false;
            (Task::none(), None)
        }
        Message::ToggleStartLocked(value) => {
            let _ = SettingsService::set_start_locked(ctx, value);
            state.start_locked = value;
            (Task::none(), None)
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let theme_row = row![
        text("Theme:"),
        button("Dark").on_press(Message::ThemeChanged(AppTheme::Dark)),
        button("Light").on_press(Message::ThemeChanged(AppTheme::Light)),
    ]
    .spacing(8);

    let mut roots_col = column![text("Library folders").size(18)].spacing(4);
    if state.library_roots.is_empty() {
        roots_col = roots_col.push(text("No folders added yet."));
    }
    for root in &state.library_roots {
        roots_col = roots_col.push(
            row![
                text(root.path.clone()).width(Length::Fill),
                button("Remove").on_press(Message::RemoveLibraryRoot(root.id)),
            ]
            .spacing(8),
        );
    }
    roots_col = roots_col.push(button("Add folder...").on_press(Message::AddLibraryRoot));

    let player_row = row![
        text_input(
            "Path to external player (e.g. mpv)",
            &state.external_player_input,
        )
        .on_input(Message::ExternalPlayerChanged),
        button("Save").on_press(Message::ExternalPlayerSave),
    ]
    .spacing(8);

    let mut privacy_col = column![text("Profile password").size(18)].spacing(4);
    if state.has_password {
        privacy_col = privacy_col
            .push(text("A password is set."))
            .push(
                checkbox("Start locked", state.start_locked).on_toggle(Message::ToggleStartLocked),
            )
            .push(button("Remove password").on_press(Message::RemovePassword));
    } else {
        privacy_col = privacy_col
            .push(
                row![
                    text_input("New password", &state.password_input)
                        .on_input(Message::PasswordInputChanged)
                        .secure(true),
                    button("Set password").on_press(Message::SetPassword),
                ]
                .spacing(8),
            )
            .push(text(
                "Set a password to enable locking and the panic shortcut.",
            ));
    }

    let mut diagnostics_col = column![text("Diagnostics").size(18)].spacing(4);
    if let Some(diagnostics) = &state.diagnostics {
        diagnostics_col = diagnostics_col
            .push(text(format!("Data dir: {}", diagnostics.data_dir)))
            .push(text(format!(
                "Applied migrations: {}",
                diagnostics.applied_migrations
            )))
            .push(text(format!(
                "ffprobe: {}",
                if diagnostics.ffprobe_available {
                    "found"
                } else {
                    "not found"
                }
            )))
            .push(text(format!(
                "ffmpeg: {}",
                if diagnostics.ffmpeg_available {
                    "found"
                } else {
                    "not found"
                }
            )));
    }

    let mut content = column![
        text("Settings").size(24),
        theme_row,
        roots_col,
        player_row,
        privacy_col,
        diagnostics_col,
    ]
    .spacing(24);
    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    container(scrollable(content)).padding(24).into()
}
