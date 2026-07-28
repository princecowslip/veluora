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
    pub metadata_encryption_enabled: bool,
    pub encryption_password_input: String,
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
            metadata_encryption_enabled: false,
            encryption_password_input: String::new(),
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
    EncryptionPasswordChanged(String),
    EnableEncryption,
    DisableEncryption,
    ExportBackup,
    BackupPathPicked(Option<PathBuf>),
    RestoreBackup,
    RestorePathPicked(Option<PathBuf>),
    ExportSupportBundle,
    SupportBundlePathPicked(Option<PathBuf>),
}

pub enum Effect {
    ThemeChanged(AppTheme),
    /// A fresh session encryption key, established by enabling metadata
    /// encryption just now (the app doesn't otherwise have one without
    /// going through the Lock screen).
    EncryptionKeyEstablished([u8; 32]),
    /// Metadata encryption was disabled — the session key (if any) is
    /// no longer meaningful.
    EncryptionKeyCleared,
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
    state.metadata_encryption_enabled =
        PrivacyService::metadata_encryption_enabled(ctx).unwrap_or(false);
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
        Message::EncryptionPasswordChanged(value) => {
            state.encryption_password_input = value;
            (Task::none(), None)
        }
        Message::EnableEncryption => {
            let password = std::mem::take(&mut state.encryption_password_input);
            match PrivacyService::enable_metadata_encryption(ctx, &password) {
                Ok(key) => {
                    state.metadata_encryption_enabled = true;
                    state.message = Some("Metadata encryption enabled.".to_string());
                    (Task::none(), Some(Effect::EncryptionKeyEstablished(key)))
                }
                Err(e) => {
                    state.message = Some(format!("Could not enable encryption: {e}"));
                    (Task::none(), None)
                }
            }
        }
        Message::DisableEncryption => {
            let password = std::mem::take(&mut state.encryption_password_input);
            match PrivacyService::disable_metadata_encryption(ctx, &password) {
                Ok(()) => {
                    state.metadata_encryption_enabled = false;
                    state.message = Some("Metadata encryption disabled.".to_string());
                    (Task::none(), Some(Effect::EncryptionKeyCleared))
                }
                Err(e) => {
                    state.message = Some(format!("Could not disable encryption: {e}"));
                    (Task::none(), None)
                }
            }
        }
        Message::ExportBackup => (
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_file_name("veloura-backup.db")
                        .save_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::BackupPathPicked,
            ),
            None,
        ),
        Message::BackupPathPicked(path) => {
            if let Some(path) = path {
                match DiagnosticsService::export_backup(ctx, &path) {
                    Ok(()) => state.message = Some(format!("Backup written to {}", path.display())),
                    Err(e) => state.message = Some(format!("Backup failed: {e}")),
                }
            }
            (Task::none(), None)
        }
        Message::RestoreBackup => (
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::RestorePathPicked,
            ),
            None,
        ),
        Message::RestorePathPicked(path) => {
            if let Some(path) = path {
                match DiagnosticsService::restore_backup(ctx, &path) {
                    Ok(()) => {
                        state.message =
                            Some("Restored. Restart Veloura to use the restored data.".to_string())
                    }
                    Err(e) => state.message = Some(format!("Restore failed: {e}")),
                }
            }
            (Task::none(), None)
        }
        Message::ExportSupportBundle => (
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_file_name("veloura-support-bundle.json")
                        .save_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::SupportBundlePathPicked,
            ),
            None,
        ),
        Message::SupportBundlePathPicked(path) => {
            if let Some(path) = path {
                let result = DiagnosticsService::support_bundle(ctx).and_then(|bundle| {
                    let json = serde_json::to_string_pretty(&bundle).unwrap_or_default();
                    std::fs::write(&path, json).map_err(application::AppError::from)
                });
                state.message = Some(match result {
                    Ok(()) => format!("Support bundle written to {}", path.display()),
                    Err(e) => format!("Could not write support bundle: {e}"),
                });
            }
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

    let mut encryption_col = column![text("Encrypted metadata").size(18)].spacing(4);
    if !state.has_password {
        encryption_col = encryption_col.push(text(
            "Set a profile password above to enable encrypted metadata.",
        ));
    } else if state.metadata_encryption_enabled {
        encryption_col = encryption_col
            .push(text(
                "Notes and private tags are encrypted at rest. Disabling requires your password.",
            ))
            .push(
                row![
                    text_input("Password", &state.encryption_password_input)
                        .on_input(Message::EncryptionPasswordChanged)
                        .secure(true),
                    button("Disable encryption").on_press(Message::DisableEncryption),
                ]
                .spacing(8),
            );
    } else {
        encryption_col = encryption_col
            .push(text(
                "Notes and private tags are stored as plain text. Enabling requires your password \
                 — the app will require unlocking on every start afterward, since that's how the \
                 encryption key is derived.",
            ))
            .push(
                row![
                    text_input("Password", &state.encryption_password_input)
                        .on_input(Message::EncryptionPasswordChanged)
                        .secure(true),
                    button("Enable encryption").on_press(Message::EnableEncryption),
                ]
                .spacing(8),
            );
    }

    let backup_col = column![
        text("Backup and restore").size(18),
        row![
            button("Export backup...").on_press(Message::ExportBackup),
            button("Restore from backup...").on_press(Message::RestoreBackup),
        ]
        .spacing(8),
    ]
    .spacing(4);

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
    diagnostics_col = diagnostics_col
        .push(button("Export support bundle...").on_press(Message::ExportSupportBundle));

    let mut content = column![
        text("Settings").size(24),
        theme_row,
        roots_col,
        player_row,
        privacy_col,
        encryption_col,
        backup_col,
        diagnostics_col,
    ]
    .spacing(24);
    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    container(scrollable(content)).padding(24).into()
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
    fn enable_encryption_requires_the_correct_password_and_yields_a_key() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "hunter2").unwrap();
        let mut state = State {
            encryption_password_input: "wrong".to_string(),
            ..State::default()
        };

        let (_, effect) = update(&mut state, &ctx, Message::EnableEncryption);
        assert!(effect.is_none());
        assert!(!state.metadata_encryption_enabled);

        state.encryption_password_input = "hunter2".to_string();
        let (_, effect) = update(&mut state, &ctx, Message::EnableEncryption);
        assert!(matches!(effect, Some(Effect::EncryptionKeyEstablished(_))));
        assert!(state.metadata_encryption_enabled);
        assert!(
            state.encryption_password_input.is_empty(),
            "the password field should be cleared after use"
        );
    }

    #[test]
    fn disable_encryption_clears_the_session_key() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "hunter2").unwrap();
        PrivacyService::enable_metadata_encryption(&ctx, "hunter2").unwrap();
        let mut state = State {
            metadata_encryption_enabled: true,
            encryption_password_input: "hunter2".to_string(),
            ..State::default()
        };

        let (_, effect) = update(&mut state, &ctx, Message::DisableEncryption);
        assert!(matches!(effect, Some(Effect::EncryptionKeyCleared)));
        assert!(!state.metadata_encryption_enabled);
    }
}
