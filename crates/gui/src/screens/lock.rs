//! Full-screen lock overlay, shown whenever the app is locked
//! (triggered by the panic shortcut or a "start locked" preference).
//! Only meaningful once a profile password is set — see
//! `docs/20-privacy-and-security.md`'s "do not invent custom
//! cryptography" rule, honored via `application::PrivacyService`'s
//! `argon2`-backed verification.

use std::sync::Arc;

use application::{AppContext, PrivacyService};
use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Length, Task};

#[derive(Default)]
pub struct State {
    pub password_input: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    Submit,
}

pub enum Effect {
    Unlocked,
}

pub fn update(
    state: &mut State,
    ctx: &Arc<AppContext>,
    message: Message,
) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::PasswordChanged(value) => {
            state.password_input = value;
            (Task::none(), None)
        }
        Message::Submit => match PrivacyService::verify_password(ctx, &state.password_input) {
            Ok(true) => {
                state.password_input.clear();
                state.error = None;
                (Task::none(), Some(Effect::Unlocked))
            }
            Ok(false) => {
                state.error = Some("Incorrect password.".to_string());
                (Task::none(), None)
            }
            Err(e) => {
                state.error = Some(e.to_string());
                (Task::none(), None)
            }
        },
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![
        text("Veloura is locked").size(28),
        text_input("Password", &state.password_input)
            .on_input(Message::PasswordChanged)
            .on_submit(Message::Submit)
            .secure(true),
        button("Unlock").on_press(Message::Submit),
    ]
    .spacing(16)
    .max_width(360);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()));
    }

    container(content)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(48)
        .into()
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
    fn wrong_password_sets_an_error_and_does_not_unlock() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "correct").unwrap();
        let mut state = State {
            password_input: "wrong".to_string(),
            ..State::default()
        };

        let (_, effect) = update(&mut state, &ctx, Message::Submit);
        assert!(effect.is_none());
        assert!(state.error.is_some());
    }

    #[test]
    fn correct_password_clears_input_and_error_and_unlocks() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "correct").unwrap();
        let mut state = State {
            password_input: "correct".to_string(),
            error: Some("stale error".to_string()),
        };

        let (_, effect) = update(&mut state, &ctx, Message::Submit);
        assert!(matches!(effect, Some(Effect::Unlocked)));
        assert!(state.password_input.is_empty());
        assert!(state.error.is_none());
    }

    #[test]
    fn submitting_with_no_password_configured_fails_closed() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            password_input: "anything".to_string(),
            ..State::default()
        };

        let (_, effect) = update(&mut state, &ctx, Message::Submit);
        assert!(effect.is_none());
        assert!(state.error.is_some());
    }
}
