//! Generic key-value settings store (`app_settings`), with typed
//! accessors for the specific keys GUI/session-level features need.
//! Kept in `application` rather than the `gui` crate per ADR-002 in
//! `docs/26-architecture-decisions.md` — a future TUI needs the same
//! onboarding/theme/lock-preference state without duplicating this
//! logic.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}

pub struct SettingsService;

impl SettingsService {
    /// Reads a raw key. Exposed for callers (like [`crate::privacy::PrivacyService`])
    /// that manage their own keys rather than going through a typed
    /// accessor below.
    pub fn get_raw(ctx: &AppContext, key: &str) -> Result<Option<String>> {
        let conn = ctx.db.connection();
        let result = conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(database::DatabaseError::from(e).into()),
        }
    }

    pub fn set_raw(ctx: &AppContext, key: &str, value: &str) -> Result<()> {
        ctx.db
            .connection()
            .execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    pub fn clear_raw(ctx: &AppContext, key: &str) -> Result<()> {
        ctx.db
            .connection()
            .execute("DELETE FROM app_settings WHERE key = ?1", params![key])
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    pub fn onboarding_complete(ctx: &AppContext) -> Result<bool> {
        Ok(Self::get_raw(ctx, "onboarding_complete")?.as_deref() == Some("true"))
    }

    pub fn set_onboarding_complete(ctx: &AppContext, complete: bool) -> Result<()> {
        Self::set_raw(ctx, "onboarding_complete", bool_str(complete))
    }

    pub fn theme(ctx: &AppContext) -> Result<Theme> {
        Ok(match Self::get_raw(ctx, "theme")?.as_deref() {
            Some("light") => Theme::Light,
            _ => Theme::Dark,
        })
    }

    pub fn set_theme(ctx: &AppContext, theme: Theme) -> Result<()> {
        Self::set_raw(
            ctx,
            "theme",
            match theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
            },
        )
    }

    pub fn external_player_path(ctx: &AppContext) -> Result<Option<String>> {
        Self::get_raw(ctx, "external_player_path")
    }

    pub fn set_external_player_path(ctx: &AppContext, path: &str) -> Result<()> {
        Self::set_raw(ctx, "external_player_path", path)
    }

    pub fn start_locked(ctx: &AppContext) -> Result<bool> {
        Ok(Self::get_raw(ctx, "start_locked")?.as_deref() == Some("true"))
    }

    pub fn set_start_locked(ctx: &AppContext, start_locked: bool) -> Result<()> {
        Self::set_raw(ctx, "start_locked", bool_str(start_locked))
    }

    pub fn last_data_cleared_at(ctx: &AppContext) -> Result<Option<String>> {
        Self::get_raw(ctx, "last_data_cleared_at")
    }

    pub fn set_last_data_cleared_at(ctx: &AppContext, timestamp: &str) -> Result<()> {
        Self::set_raw(ctx, "last_data_cleared_at", timestamp)
    }

    /// The cache eviction ceiling in bytes — `None` means unlimited (the
    /// default). See `PrivacyService::enforce_cache_quota`.
    pub fn cache_quota_bytes(ctx: &AppContext) -> Result<Option<u64>> {
        Ok(Self::get_raw(ctx, "cache_quota_bytes")?.and_then(|s| s.parse().ok()))
    }

    pub fn set_cache_quota_bytes(ctx: &AppContext, quota: Option<u64>) -> Result<()> {
        match quota {
            Some(bytes) => Self::set_raw(ctx, "cache_quota_bytes", &bytes.to_string()),
            None => Self::clear_raw(ctx, "cache_quota_bytes"),
        }
    }

    /// The permanent-download eviction ceiling in bytes — `None` means
    /// unlimited (the default). See `PrivacyService::enforce_download_quota`.
    pub fn download_quota_bytes(ctx: &AppContext) -> Result<Option<u64>> {
        Ok(Self::get_raw(ctx, "download_quota_bytes")?.and_then(|s| s.parse().ok()))
    }

    pub fn set_download_quota_bytes(ctx: &AppContext, quota: Option<u64>) -> Result<()> {
        match quota {
            Some(bytes) => Self::set_raw(ctx, "download_quota_bytes", &bytes.to_string()),
            None => Self::clear_raw(ctx, "download_quota_bytes"),
        }
    }

    /// The destination-path template `DownloadService::add` renders a
    /// completed download's filename from. Supported tokens: `{title}`,
    /// `{source}`, `{source_id}`, `{item_id}`, `{sequence}`, `{ext}`.
    pub fn download_naming_template(ctx: &AppContext) -> Result<String> {
        Ok(Self::get_raw(ctx, "download_naming_template")?
            .unwrap_or_else(|| "{source}/{title} [{item_id}]/{sequence}.{ext}".to_string()))
    }

    pub fn set_download_naming_template(ctx: &AppContext, template: &str) -> Result<()> {
        Self::set_raw(ctx, "download_naming_template", template)
    }
}

fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_complete_defaults_to_false() {
        let ctx = AppContext::open_in_memory().unwrap();
        assert!(!SettingsService::onboarding_complete(&ctx).unwrap());
    }

    #[test]
    fn onboarding_complete_round_trips() {
        let ctx = AppContext::open_in_memory().unwrap();
        SettingsService::set_onboarding_complete(&ctx, true).unwrap();
        assert!(SettingsService::onboarding_complete(&ctx).unwrap());
        SettingsService::set_onboarding_complete(&ctx, false).unwrap();
        assert!(!SettingsService::onboarding_complete(&ctx).unwrap());
    }

    #[test]
    fn theme_defaults_to_dark_and_round_trips() {
        let ctx = AppContext::open_in_memory().unwrap();
        assert_eq!(SettingsService::theme(&ctx).unwrap(), Theme::Dark);
        SettingsService::set_theme(&ctx, Theme::Light).unwrap();
        assert_eq!(SettingsService::theme(&ctx).unwrap(), Theme::Light);
    }

    #[test]
    fn external_player_path_is_none_until_set() {
        let ctx = AppContext::open_in_memory().unwrap();
        assert_eq!(SettingsService::external_player_path(&ctx).unwrap(), None);
        SettingsService::set_external_player_path(&ctx, "/usr/bin/mpv").unwrap();
        assert_eq!(
            SettingsService::external_player_path(&ctx).unwrap(),
            Some("/usr/bin/mpv".to_string())
        );
    }

    #[test]
    fn set_raw_upserts_not_duplicates() {
        let ctx = AppContext::open_in_memory().unwrap();
        SettingsService::set_raw(&ctx, "k", "v1").unwrap();
        SettingsService::set_raw(&ctx, "k", "v2").unwrap();
        assert_eq!(
            SettingsService::get_raw(&ctx, "k").unwrap(),
            Some("v2".to_string())
        );
        let count: i64 = ctx
            .db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM app_settings WHERE key = 'k'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn download_quota_bytes_defaults_to_unlimited_and_round_trips() {
        let ctx = AppContext::open_in_memory().unwrap();
        assert_eq!(SettingsService::download_quota_bytes(&ctx).unwrap(), None);
        SettingsService::set_download_quota_bytes(&ctx, Some(1024)).unwrap();
        assert_eq!(
            SettingsService::download_quota_bytes(&ctx).unwrap(),
            Some(1024)
        );
        SettingsService::set_download_quota_bytes(&ctx, None).unwrap();
        assert_eq!(SettingsService::download_quota_bytes(&ctx).unwrap(), None);
    }

    #[test]
    fn download_naming_template_has_a_sensible_default_and_round_trips() {
        let ctx = AppContext::open_in_memory().unwrap();
        assert_eq!(
            SettingsService::download_naming_template(&ctx).unwrap(),
            "{source}/{title} [{item_id}]/{sequence}.{ext}"
        );
        SettingsService::set_download_naming_template(&ctx, "{title}.{ext}").unwrap();
        assert_eq!(
            SettingsService::download_naming_template(&ctx).unwrap(),
            "{title}.{ext}"
        );
    }

    #[test]
    fn clear_raw_removes_the_key() {
        let ctx = AppContext::open_in_memory().unwrap();
        SettingsService::set_raw(&ctx, "k", "v").unwrap();
        SettingsService::clear_raw(&ctx, "k").unwrap();
        assert_eq!(SettingsService::get_raw(&ctx, "k").unwrap(), None);
    }
}
