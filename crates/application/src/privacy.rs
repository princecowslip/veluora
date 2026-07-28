//! Profile-password locking and local data/cache management for the
//! Privacy Center screen.
//!
//! Password storage uses `argon2` (the PHC-recommended password hashing
//! function) rather than any custom scheme, per
//! `docs/20-privacy-and-security.md`'s "do not invent custom
//! cryptography for password storage" rule.

use std::fs;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::settings::SettingsService;
use crate::time_format::to_rfc3339;

const PASSWORD_HASH_KEY: &str = "lock_password_hash";

pub struct PrivacyService;

impl PrivacyService {
    pub fn has_password(ctx: &AppContext) -> Result<bool> {
        Ok(SettingsService::get_raw(ctx, PASSWORD_HASH_KEY)?.is_some())
    }

    /// Hashes and stores `password`, replacing any existing one.
    pub fn set_password(ctx: &AppContext, password: &str) -> Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::InvalidPath(format!("could not hash password: {e}")))?
            .to_string();
        SettingsService::set_raw(ctx, PASSWORD_HASH_KEY, &hash)
    }

    pub fn remove_password(ctx: &AppContext) -> Result<()> {
        SettingsService::clear_raw(ctx, PASSWORD_HASH_KEY)
    }

    /// Returns `false` (never an error) when no password is set —
    /// there's nothing to verify against, so verification simply fails
    /// closed.
    pub fn verify_password(ctx: &AppContext, password: &str) -> Result<bool> {
        let Some(stored) = SettingsService::get_raw(ctx, PASSWORD_HASH_KEY)? else {
            return Ok(false);
        };
        let parsed = PasswordHash::new(&stored)
            .map_err(|e| AppError::InvalidPath(format!("stored password hash is corrupt: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    /// Sums the size of every file under `<data_dir>/cache` — `0` if the
    /// directory doesn't exist yet (nothing has been cached).
    pub fn cache_size_bytes(ctx: &AppContext) -> Result<u64> {
        let cache_dir = ctx.data_dir.join("cache");
        if !cache_dir.exists() {
            return Ok(0);
        }
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        Ok(total)
    }

    /// Deletes and recreates `<data_dir>/cache` (thumbnails regenerate
    /// on demand, same as their normal lazy-generation path), and
    /// records when this happened.
    pub fn clear_cache(ctx: &AppContext) -> Result<()> {
        let cache_dir = ctx.data_dir.join("cache");
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)?;
        }
        fs::create_dir_all(&cache_dir)?;
        SettingsService::set_last_data_cleared_at(ctx, &to_rfc3339(time::OffsetDateTime::now_utc()))
    }

    /// Wipes every locally stored preference, playback/reading state,
    /// and cache file. Leaves scanned library items and their metadata
    /// intact — this clears history/preferences/cache, not the library
    /// itself.
    pub fn delete_all_local_data(ctx: &AppContext) -> Result<()> {
        Self::clear_cache(ctx)?;
        ctx.db
            .connection()
            .execute("DELETE FROM user_state", [])
            .map_err(database::DatabaseError::from)?;
        ctx.db
            .connection()
            .execute("DELETE FROM app_settings", [])
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> (AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        (ctx, dir)
    }

    #[test]
    fn no_password_set_by_default_and_verification_fails_closed() {
        let (ctx, _dir) = test_ctx();
        assert!(!PrivacyService::has_password(&ctx).unwrap());
        assert!(!PrivacyService::verify_password(&ctx, "anything").unwrap());
    }

    #[test]
    fn set_password_then_verify_round_trips() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "correct horse battery staple").unwrap();
        assert!(PrivacyService::has_password(&ctx).unwrap());
        assert!(PrivacyService::verify_password(&ctx, "correct horse battery staple").unwrap());
        assert!(!PrivacyService::verify_password(&ctx, "wrong password").unwrap());
    }

    #[test]
    fn remove_password_clears_it() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "hunter2").unwrap();
        PrivacyService::remove_password(&ctx).unwrap();
        assert!(!PrivacyService::has_password(&ctx).unwrap());
    }

    #[test]
    fn cache_size_is_zero_before_anything_is_cached() {
        let (ctx, _dir) = test_ctx();
        assert_eq!(PrivacyService::cache_size_bytes(&ctx).unwrap(), 0);
    }

    #[test]
    fn cache_size_reflects_real_files_and_clear_cache_empties_it() {
        let (ctx, _dir) = test_ctx();
        let cache_dir = ctx.data_dir.join("cache").join("thumbnails");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("a.jpg"), vec![0u8; 1024]).unwrap();

        assert_eq!(PrivacyService::cache_size_bytes(&ctx).unwrap(), 1024);

        PrivacyService::clear_cache(&ctx).unwrap();
        assert_eq!(PrivacyService::cache_size_bytes(&ctx).unwrap(), 0);
        assert!(SettingsService::last_data_cleared_at(&ctx)
            .unwrap()
            .is_some());
    }

    #[test]
    fn delete_all_local_data_wipes_settings_and_user_state_but_keeps_items() {
        let (ctx, _dir) = test_ctx();
        let item_id = domain::ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Kept', 'unrated', datetime('now'), datetime('now'))",
                rusqlite::params![item_id.to_string()],
            )
            .unwrap();
        crate::user_state::UserStateService::set_favorite(&ctx, item_id, true).unwrap();
        SettingsService::set_theme(&ctx, crate::settings::Theme::Light).unwrap();

        PrivacyService::delete_all_local_data(&ctx).unwrap();

        let item_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(item_count, 1, "library items must survive a data wipe");

        let user_state_count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM user_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(user_state_count, 0);

        assert_eq!(
            SettingsService::theme(&ctx).unwrap(),
            crate::settings::Theme::Dark
        );
    }
}
