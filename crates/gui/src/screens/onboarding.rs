//! First-run flow: welcome -> pick a library folder -> initial scan ->
//! privacy-defaults summary -> finish. "Skip setup" is available at
//! every step and lands on an empty Home, which is already a supported
//! state (`LibraryService::status` returns zero counts cleanly).

use std::path::PathBuf;
use std::sync::Arc;

use application::{AppContext, ScanReport};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Task};

use crate::app::Screen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Welcome,
    PickFolder,
    Scanning,
    PrivacySummary,
}

pub struct State {
    pub step: Step,
    pub folder: Option<PathBuf>,
    pub scan_report: Option<ScanReport>,
    pub error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            step: Step::Welcome,
            folder: None,
            scan_report: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Back,
    Skip,
    PickFolder,
    FolderPicked(Option<PathBuf>),
    ScanFinished(Result<ScanReport, String>),
    Finish,
}

pub enum Effect {
    Navigate(Screen),
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    message: Message,
) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::Next => {
            state.step = match state.step {
                Step::Welcome => Step::PickFolder,
                Step::PickFolder => Step::Scanning,
                Step::Scanning | Step::PrivacySummary => Step::PrivacySummary,
            };
            (Task::none(), None)
        }
        Message::Back => {
            state.step = match state.step {
                Step::Welcome | Step::PickFolder => Step::Welcome,
                Step::Scanning => Step::PickFolder,
                Step::PrivacySummary => Step::Scanning,
            };
            (Task::none(), None)
        }
        Message::Skip => {
            let _ = application::SettingsService::set_onboarding_complete(ctx, true);
            (Task::none(), Some(Effect::Navigate(Screen::Home)))
        }
        Message::PickFolder => (
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
            if let Some(path) = &path {
                let _ = application::LibraryRootService::add(ctx, path, None);
            }
            state.folder = path;
            state.step = Step::Scanning;
            let ctx = ctx.clone();
            (
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            application::ScanService::scan_all(&ctx)
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r.map_err(|e| e.to_string()))
                    },
                    Message::ScanFinished,
                ),
                None,
            )
        }
        Message::ScanFinished(result) => {
            match result {
                Ok(report) => state.scan_report = Some(report),
                Err(e) => state.error = Some(e),
            }
            state.step = Step::PrivacySummary;
            (Task::none(), None)
        }
        Message::Finish => {
            let _ = application::SettingsService::set_onboarding_complete(ctx, true);
            (Task::none(), Some(Effect::Navigate(Screen::Home)))
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let content: Element<'_, Message> = match state.step {
        Step::Welcome => column![
            text("Welcome to Veloura").size(28),
            text("A private, local-first media library."),
            row![
                button("Skip setup").on_press(Message::Skip),
                button("Get started").on_press(Message::Next),
            ]
            .spacing(12),
        ]
        .spacing(16)
        .into(),
        Step::PickFolder => {
            let folder_label = state
                .folder
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "No folder selected yet".to_string());
            column![
                text("Add your first library folder").size(24),
                text(folder_label),
                row![
                    button("Choose folder...").on_press(Message::PickFolder),
                    button("Back").on_press(Message::Back),
                    button("Skip setup").on_press(Message::Skip),
                ]
                .spacing(12),
            ]
            .spacing(16)
            .into()
        }
        Step::Scanning => column![text("Scanning your library...").size(24)]
            .spacing(16)
            .into(),
        Step::PrivacySummary => {
            let summary = state
                .scan_report
                .as_ref()
                .map(|r| {
                    let added: u32 = r.roots.iter().map(|root| root.added).sum();
                    format!("Added {added} item(s) to your library.")
                })
                .unwrap_or_else(|| "No folder was scanned.".to_string());
            column![
                text("You're all set").size(24),
                text(summary),
                text("Veloura is private by default:"),
                text("- No account required"),
                text("- Telemetry is off"),
                text("- The local API only listens on this device"),
                row![
                    button("Back").on_press(Message::Back),
                    button("Finish").on_press(Message::Finish),
                ]
                .spacing(12),
            ]
            .spacing(16)
            .into()
        }
    };
    container(content).padding(32).into()
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
    fn next_advances_through_every_step_in_order() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        assert_eq!(state.step, Step::Welcome);

        let _ = update(&mut state, &ctx, Message::Next);
        assert_eq!(state.step, Step::PickFolder);

        let _ = update(&mut state, &ctx, Message::Next);
        assert_eq!(state.step, Step::Scanning);

        let _ = update(&mut state, &ctx, Message::Next);
        assert_eq!(state.step, Step::PrivacySummary);

        // Next is a no-op on the final step.
        let _ = update(&mut state, &ctx, Message::Next);
        assert_eq!(state.step, Step::PrivacySummary);
    }

    #[test]
    fn back_reverses_next() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            step: Step::PrivacySummary,
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::Back);
        assert_eq!(state.step, Step::Scanning);
        let _ = update(&mut state, &ctx, Message::Back);
        assert_eq!(state.step, Step::PickFolder);
        let _ = update(&mut state, &ctx, Message::Back);
        assert_eq!(state.step, Step::Welcome);
        // Back is a no-op on the first step.
        let _ = update(&mut state, &ctx, Message::Back);
        assert_eq!(state.step, Step::Welcome);
    }

    #[test]
    fn skip_marks_onboarding_complete_and_navigates_home() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();

        let (_, effect) = update(&mut state, &ctx, Message::Skip);
        assert!(matches!(effect, Some(Effect::Navigate(Screen::Home))));
        assert!(application::SettingsService::onboarding_complete(&ctx).unwrap());
    }

    #[test]
    fn finish_marks_onboarding_complete_and_navigates_home() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();

        let (_, effect) = update(&mut state, &ctx, Message::Finish);
        assert!(matches!(effect, Some(Effect::Navigate(Screen::Home))));
        assert!(application::SettingsService::onboarding_complete(&ctx).unwrap());
    }
}
