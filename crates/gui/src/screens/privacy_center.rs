//! Privacy Center: lock status, cache size, telemetry status (always
//! off), and data/cache deletion controls — per `docs/06-ui-ux.md`'s
//! Privacy Center spec. Stored source credentials are honestly reported
//! as empty; no connectors exist yet.

use std::sync::Arc;

use application::{AppContext, CacheBreakdown, PrivacyService, SettingsService};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Task};

#[derive(Default)]
pub struct State {
    pub cache_breakdown: CacheBreakdown,
    pub quota_bytes: Option<u64>,
    pub quota_input: String,
    pub has_password: bool,
    pub last_cleared_at: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ClearCache,
    DeleteAllData,
    QuotaInputChanged(String),
    SetQuota,
    ClearQuota,
    EnforceQuotaNow,
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.cache_breakdown = PrivacyService::cache_breakdown(ctx).unwrap_or_default();
    state.quota_bytes = SettingsService::cache_quota_bytes(ctx).ok().flatten();
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
        Message::QuotaInputChanged(value) => {
            state.quota_input = value;
            return Task::none();
        }
        Message::SetQuota => match state.quota_input.trim().parse::<u64>() {
            Ok(mb) => match SettingsService::set_cache_quota_bytes(ctx, Some(mb * 1024 * 1024)) {
                Ok(()) => state.message = Some(format!("Cache quota set to {mb} MB.")),
                Err(e) => state.message = Some(format!("Could not set quota: {e}")),
            },
            Err(_) => state.message = Some("Enter a whole number of MB for the quota.".to_string()),
        },
        Message::ClearQuota => match SettingsService::set_cache_quota_bytes(ctx, None) {
            Ok(()) => state.message = Some("Cache quota cleared (unlimited).".to_string()),
            Err(e) => state.message = Some(format!("Could not clear quota: {e}")),
        },
        Message::EnforceQuotaNow => match PrivacyService::enforce_cache_quota(ctx) {
            Ok(report) => {
                state.message = Some(format!(
                    "Evicted {} file(s), {:.2} MB freed.",
                    report.evicted_files,
                    report.evicted_bytes as f64 / (1024.0 * 1024.0)
                ))
            }
            Err(e) => state.message = Some(format!("Could not enforce quota: {e}")),
        },
    }
    refresh(state, ctx);
    Task::none()
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

pub fn view(state: &State) -> Element<'_, Message> {
    let lock_status = if state.has_password {
        "A profile password is set; locking and the panic shortcut (Ctrl+Shift+L) are enabled."
    } else {
        "No profile password is set — set one in Settings to enable locking and the panic shortcut."
    };

    let mut content = column![
        text("Privacy Center").size(24),
        text(lock_status),
        text(format!(
            "Cache size: {:.2} MB (thumbnails: {:.2} MB, stories: {:.2} MB, other: {:.2} MB)",
            mb(state.cache_breakdown.total_bytes),
            mb(state.cache_breakdown.thumbnails_bytes),
            mb(state.cache_breakdown.stories_bytes),
            mb(state.cache_breakdown.other_bytes),
        )),
        button("Clear cache").on_press(Message::ClearCache),
        text(match state.quota_bytes {
            Some(bytes) => format!("Cache quota: {:.0} MB", mb(bytes)),
            None => "Cache quota: unlimited".to_string(),
        }),
        row![
            text_input("Quota in MB", &state.quota_input).on_input(Message::QuotaInputChanged),
            button("Set quota").on_press(Message::SetQuota),
            button("Clear quota").on_press(Message::ClearQuota),
        ]
        .spacing(8),
        button("Enforce quota now").on_press(Message::EnforceQuotaNow),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    #[test]
    fn quota_input_changed_updates_the_input_without_touching_the_stored_quota() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        let _ = update(
            &mut state,
            &ctx,
            Message::QuotaInputChanged("50".to_string()),
        );
        assert_eq!(state.quota_input, "50");
        assert_eq!(state.quota_bytes, None);
    }

    #[test]
    fn set_quota_parses_megabytes_and_persists_the_byte_value() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        state.quota_input = "50".to_string();

        let _ = update(&mut state, &ctx, Message::SetQuota);

        assert_eq!(state.quota_bytes, Some(50 * 1024 * 1024));
        assert_eq!(
            SettingsService::cache_quota_bytes(&ctx).unwrap(),
            Some(50 * 1024 * 1024)
        );
    }

    #[test]
    fn set_quota_with_non_numeric_input_reports_a_message_and_sets_nothing() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();
        state.quota_input = "not a number".to_string();

        let _ = update(&mut state, &ctx, Message::SetQuota);

        assert_eq!(state.quota_bytes, None);
        assert!(state.message.unwrap().contains("whole number"));
    }

    #[test]
    fn clear_quota_removes_a_previously_set_quota() {
        let (ctx, _dir) = test_ctx();
        SettingsService::set_cache_quota_bytes(&ctx, Some(1024)).unwrap();
        let mut state = State::default();

        let _ = update(&mut state, &ctx, Message::ClearQuota);

        assert_eq!(state.quota_bytes, None);
        assert_eq!(SettingsService::cache_quota_bytes(&ctx).unwrap(), None);
    }

    #[test]
    fn enforce_quota_now_reports_eviction_results() {
        let (ctx, _dir) = test_ctx();
        let mut state = State::default();

        let _ = update(&mut state, &ctx, Message::EnforceQuotaNow);

        assert!(state.message.unwrap().contains("Evicted 0 file(s)"));
    }
}
