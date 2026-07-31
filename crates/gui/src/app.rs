//! Top-level application state and dispatch. Wraps every screen: when
//! `locked` is true, the lock screen renders instead of the active
//! screen's content, regardless of navigation state.

use std::sync::Arc;

use application::AppContext;
use domain::ItemId;
use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{button, column, row};
use iced::{Element, Subscription, Task, Theme};

use crate::screens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Onboarding,
    Home,
    Library,
    Viewer,
    PrivacyCenter,
    Sources,
    Discover,
    Downloads,
    Settings,
}

pub struct App {
    ctx: Arc<AppContext>,
    screen: Screen,
    locked: bool,
    theme: application::Theme,
    /// Session-only AES-256 key for encrypted metadata (never
    /// persisted) — `None` until the user unlocks with the profile
    /// password (or enables encryption in Settings), and always `None`
    /// if metadata encryption isn't enabled at all.
    encryption_key: Option<[u8; 32]>,

    onboarding: screens::onboarding::State,
    home: screens::home::State,
    library: screens::library::State,
    viewer: screens::viewer::State,
    privacy_center: screens::privacy_center::State,
    sources: screens::sources::State,
    discover: screens::discover::State,
    downloads: screens::downloads::State,
    settings: screens::settings::State,
    lock: screens::lock::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Screen),
    PanicLock,
    Onboarding(screens::onboarding::Message),
    Home(screens::home::Message),
    Library(screens::library::Message),
    Viewer(screens::viewer::Message),
    PrivacyCenter(screens::privacy_center::Message),
    Sources(screens::sources::Message),
    Discover(screens::discover::Message),
    Downloads(screens::downloads::Message),
    Settings(screens::settings::Message),
    Lock(screens::lock::Message),
}

impl App {
    pub fn new(ctx: Arc<AppContext>) -> (Self, Task<Message>) {
        let onboarding_done =
            application::SettingsService::onboarding_complete(&ctx).unwrap_or(false);
        let has_password = application::PrivacyService::has_password(&ctx).unwrap_or(false);
        let metadata_encryption_enabled =
            application::PrivacyService::metadata_encryption_enabled(&ctx).unwrap_or(false);
        // Metadata encryption always forces a start-locked boot,
        // regardless of the separate `start_locked` preference — there's
        // no session key without the password, so notes/private_tags
        // couldn't be decrypted otherwise.
        let start_locked = has_password
            && (metadata_encryption_enabled
                || application::SettingsService::start_locked(&ctx).unwrap_or(false));
        let theme = application::SettingsService::theme(&ctx).unwrap_or(application::Theme::Dark);

        let mut app = App {
            ctx,
            screen: if onboarding_done {
                Screen::Home
            } else {
                Screen::Onboarding
            },
            locked: start_locked,
            theme,
            encryption_key: None,
            onboarding: screens::onboarding::State::default(),
            home: screens::home::State::default(),
            library: screens::library::State::default(),
            viewer: screens::viewer::State::default(),
            privacy_center: screens::privacy_center::State::default(),
            sources: screens::sources::State::default(),
            discover: screens::discover::State::default(),
            downloads: screens::downloads::State::default(),
            settings: screens::settings::State::default(),
            lock: screens::lock::State::default(),
        };
        app.refresh_active_screen();
        (app, Task::none())
    }

    fn refresh_active_screen(&mut self) {
        match self.screen {
            Screen::Home => screens::home::refresh(&mut self.home, &self.ctx),
            Screen::Library => screens::library::refresh(&mut self.library, &self.ctx),
            Screen::PrivacyCenter => {
                screens::privacy_center::refresh(&mut self.privacy_center, &self.ctx)
            }
            Screen::Sources => screens::sources::refresh(&mut self.sources, &self.ctx),
            Screen::Settings => screens::settings::refresh(&mut self.settings, &self.ctx),
            Screen::Discover => screens::discover::refresh(&mut self.discover, &self.ctx),
            Screen::Downloads => screens::downloads::refresh(&mut self.downloads, &self.ctx),
            Screen::Onboarding | Screen::Viewer => {}
        }
    }

    fn navigate(&mut self, screen: Screen) {
        self.screen = screen;
        self.refresh_active_screen();
    }

    fn open_item(&mut self, item_id: ItemId) {
        screens::viewer::load(
            &mut self.viewer,
            &self.ctx,
            item_id,
            self.encryption_key.as_ref(),
        );
        self.screen = Screen::Viewer;
    }

    pub fn title(&self) -> String {
        "Veloura".to_string()
    }

    pub fn theme(&self) -> Theme {
        crate::theme::from_app_theme(self.theme)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![keyboard::on_key_press(panic_shortcut)];
        // Live progress: only while the Downloads screen is showing
        // *and* something is actually in flight — there's no
        // server-push event stream for downloads (`local-api` has no
        // SSE/WebSocket support), so this is a polling fallback.
        if self.screen == Screen::Downloads
            && screens::downloads::has_in_flight_downloads(&self.downloads)
        {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::Downloads(screens::downloads::Message::Refresh)),
            );
        }
        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(screen) => {
                self.navigate(screen);
                Task::none()
            }
            Message::PanicLock => {
                if application::PrivacyService::has_password(&self.ctx).unwrap_or(false) {
                    self.locked = true;
                }
                Task::none()
            }
            Message::Lock(msg) => {
                let (task, effect) = screens::lock::update(&mut self.lock, &self.ctx, msg);
                if let Some(screens::lock::Effect::Unlocked(key)) = effect {
                    self.locked = false;
                    self.encryption_key = key;
                    self.navigate(self.screen);
                }
                task.map(Message::Lock)
            }
            Message::Onboarding(msg) => {
                let (task, effect) =
                    screens::onboarding::update(&mut self.onboarding, &self.ctx, msg);
                if let Some(screens::onboarding::Effect::Navigate(screen)) = effect {
                    self.navigate(screen);
                }
                task.map(Message::Onboarding)
            }
            Message::Home(msg) => {
                let (task, effect) = screens::home::update(&mut self.home, msg);
                match effect {
                    Some(screens::home::Effect::SearchLibrary(query)) => {
                        screens::library::set_query(&mut self.library, &self.ctx, query);
                        self.screen = Screen::Library;
                    }
                    Some(screens::home::Effect::OpenItem(item_id)) => self.open_item(item_id),
                    None => {}
                }
                task.map(Message::Home)
            }
            Message::Library(msg) => {
                let (task, effect) = screens::library::update(&mut self.library, &self.ctx, msg);
                if let Some(screens::library::Effect::OpenItem(item_id)) = effect {
                    self.open_item(item_id);
                }
                task.map(Message::Library)
            }
            Message::Viewer(msg) => {
                let (task, effect) = screens::viewer::update(
                    &mut self.viewer,
                    &self.ctx,
                    self.encryption_key.as_ref(),
                    msg,
                );
                if let Some(screens::viewer::Effect::Deleted) = effect {
                    self.navigate(Screen::Library);
                }
                task.map(Message::Viewer)
            }
            Message::PrivacyCenter(msg) => {
                screens::privacy_center::update(&mut self.privacy_center, &self.ctx, msg)
                    .map(Message::PrivacyCenter)
            }
            Message::Sources(msg) => {
                screens::sources::update(&mut self.sources, &self.ctx, msg).map(Message::Sources)
            }
            Message::Discover(msg) => {
                screens::discover::update(&mut self.discover, &self.ctx, msg).map(Message::Discover)
            }
            Message::Downloads(msg) => {
                screens::downloads::update(&mut self.downloads, &self.ctx, msg)
                    .map(Message::Downloads)
            }
            Message::Settings(msg) => {
                let (task, effect) = screens::settings::update(&mut self.settings, &self.ctx, msg);
                match effect {
                    Some(screens::settings::Effect::ThemeChanged(theme)) => self.theme = theme,
                    Some(screens::settings::Effect::EncryptionKeyEstablished(key)) => {
                        self.encryption_key = Some(key);
                    }
                    Some(screens::settings::Effect::EncryptionKeyCleared) => {
                        self.encryption_key = None;
                    }
                    None => {}
                }
                task.map(Message::Settings)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.locked {
            return screens::lock::view(&self.lock).map(Message::Lock);
        }

        if self.screen == Screen::Onboarding {
            return screens::onboarding::view(&self.onboarding).map(Message::Onboarding);
        }

        let body: Element<'_, Message> = match self.screen {
            Screen::Onboarding => unreachable!(),
            Screen::Home => screens::home::view(&self.home).map(Message::Home),
            Screen::Library => screens::library::view(&self.library).map(Message::Library),
            Screen::Viewer => screens::viewer::view(&self.viewer).map(Message::Viewer),
            Screen::PrivacyCenter => {
                screens::privacy_center::view(&self.privacy_center).map(Message::PrivacyCenter)
            }
            Screen::Sources => screens::sources::view(&self.sources).map(Message::Sources),
            Screen::Discover => screens::discover::view(&self.discover).map(Message::Discover),
            Screen::Downloads => screens::downloads::view(&self.downloads).map(Message::Downloads),
            Screen::Settings => screens::settings::view(&self.settings).map(Message::Settings),
        };

        column![self.nav_bar(), body].into()
    }

    fn nav_bar(&self) -> Element<'_, Message> {
        row![
            button("Home").on_press(Message::Navigate(Screen::Home)),
            button("Library").on_press(Message::Navigate(Screen::Library)),
            button("Privacy Center").on_press(Message::Navigate(Screen::PrivacyCenter)),
            button("Sources").on_press(Message::Navigate(Screen::Sources)),
            button("Discover").on_press(Message::Navigate(Screen::Discover)),
            button("Downloads").on_press(Message::Navigate(Screen::Downloads)),
            button("Settings").on_press(Message::Navigate(Screen::Settings)),
        ]
        .spacing(12)
        .padding(12)
        .into()
    }
}

/// The panic shortcut (Ctrl+Shift+L): locks the app immediately.
/// Only takes effect when a profile password is configured — there's
/// nothing to gate the lock without one (see `docs/20-privacy-and-security.md`).
fn panic_shortcut(key: Key, modifiers: Modifiers) -> Option<Message> {
    if modifiers.control() && modifiers.shift() {
        if let Key::Character(c) = key.as_ref() {
            if c.eq_ignore_ascii_case("l") {
                return Some(Message::PanicLock);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        let (app, _task) = App::new(ctx);
        (app, dir)
    }

    #[test]
    fn panic_shortcut_fires_on_ctrl_shift_l() {
        let modifiers = Modifiers::CTRL | Modifiers::SHIFT;
        assert!(matches!(
            panic_shortcut(Key::Character("l".into()), modifiers),
            Some(Message::PanicLock)
        ));
        assert!(matches!(
            panic_shortcut(Key::Character("L".into()), modifiers),
            Some(Message::PanicLock)
        ));
    }

    #[test]
    fn panic_shortcut_ignores_other_keys_and_modifier_combos() {
        assert!(panic_shortcut(Key::Character("l".into()), Modifiers::CTRL).is_none());
        assert!(panic_shortcut(Key::Character("l".into()), Modifiers::SHIFT).is_none());
        assert!(panic_shortcut(
            Key::Character("k".into()),
            Modifiers::CTRL | Modifiers::SHIFT
        )
        .is_none());
    }

    #[test]
    fn a_fresh_context_starts_on_onboarding_and_unlocked() {
        let (app, _dir) = test_app();
        assert_eq!(app.screen, Screen::Onboarding);
        assert!(!app.locked);
    }

    #[test]
    fn onboarding_completion_persists_across_a_new_app_instance() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        application::SettingsService::set_onboarding_complete(&ctx, true).unwrap();

        let (app, _task) = App::new(ctx);
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn navigate_switches_the_active_screen() {
        let (mut app, _dir) = test_app();
        app.navigate(Screen::Settings);
        assert_eq!(app.screen, Screen::Settings);
    }

    #[test]
    fn a_password_protected_context_that_prefers_start_locked_boots_locked() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        application::PrivacyService::set_password(&ctx, "hunter2").unwrap();
        application::SettingsService::set_start_locked(&ctx, true).unwrap();

        let (app, _task) = App::new(ctx);
        assert!(app.locked);
    }

    #[test]
    fn metadata_encryption_forces_a_locked_boot_even_without_the_start_locked_preference() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        application::PrivacyService::set_password(&ctx, "hunter2").unwrap();
        application::PrivacyService::enable_metadata_encryption(&ctx, "hunter2").unwrap();
        // start_locked is deliberately left at its default (false).
        assert!(!application::SettingsService::start_locked(&ctx).unwrap());

        let (app, _task) = App::new(ctx);
        assert!(
            app.locked,
            "encryption needs the password every session to derive the key, regardless of start_locked"
        );
    }

    #[test]
    fn panic_lock_message_locks_only_when_a_password_is_set() {
        let (mut app, _dir) = test_app();
        let _ = app.update(Message::PanicLock);
        assert!(
            !app.locked,
            "no password configured, so the panic shortcut has nothing to gate"
        );

        application::PrivacyService::set_password(&app.ctx.clone(), "hunter2").unwrap();
        let _ = app.update(Message::PanicLock);
        assert!(app.locked);
    }

    #[test]
    fn unlocking_with_the_correct_password_clears_the_lock() {
        let (mut app, _dir) = test_app();
        application::PrivacyService::set_password(&app.ctx.clone(), "hunter2").unwrap();
        app.locked = true;

        app.lock.password_input = "hunter2".to_string();
        let _ = app.update(Message::Lock(screens::lock::Message::Submit));
        assert!(!app.locked);
    }
}
