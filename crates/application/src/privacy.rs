//! Profile-password locking and local data/cache management for the
//! Privacy Center screen.
//!
//! Password storage uses `argon2` (the PHC-recommended password hashing
//! function) rather than any custom scheme, per
//! `docs/20-privacy-and-security.md`'s "do not invent custom
//! cryptography for password storage" rule.

use std::fs;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::settings::SettingsService;
use crate::time_format::to_rfc3339;

const PASSWORD_HASH_KEY: &str = "lock_password_hash";
const METADATA_ENCRYPTION_SALT_KEY: &str = "metadata_encryption_salt";
const METADATA_ENCRYPTION_ENABLED_KEY: &str = "metadata_encryption_enabled";
/// Marks a stored value as AES-256-GCM ciphertext (base64 of nonce ||
/// ciphertext) rather than plaintext, so `decrypt_text` can pass
/// unmarked plaintext through unchanged.
const ENCRYPTED_PREFIX: &str = "enc:v1:";

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

    // --- Metadata encryption ---
    //
    // App-level field encryption for `user_state.notes`/`private_tags`
    // only — not whole-database encryption. Chosen over SQLCipher to
    // avoid a native crypto dependency and cross-platform build risk;
    // see the Milestone E plan for the full rationale. Key material is
    // never persisted — only a per-profile salt is stored, and the
    // AES-256 key is derived fresh from the profile password each
    // session (held in memory by the caller, not by this service).

    pub fn metadata_encryption_enabled(ctx: &AppContext) -> Result<bool> {
        Ok(
            SettingsService::get_raw(ctx, METADATA_ENCRYPTION_ENABLED_KEY)?.as_deref()
                == Some("true"),
        )
    }

    /// Derives the AES-256 key for `password`, generating and persisting
    /// a random salt first if this is the first derivation.
    pub fn derive_key(ctx: &AppContext, password: &str) -> Result<[u8; 32]> {
        let salt_b64 = match SettingsService::get_raw(ctx, METADATA_ENCRYPTION_SALT_KEY)? {
            Some(existing) => existing,
            None => {
                let mut salt = [0u8; 16];
                OsRng.fill_bytes(&mut salt);
                let encoded = BASE64.encode(salt);
                SettingsService::set_raw(ctx, METADATA_ENCRYPTION_SALT_KEY, &encoded)?;
                encoded
            }
        };
        let salt = BASE64
            .decode(&salt_b64)
            .map_err(|e| AppError::InvalidPath(format!("corrupt encryption salt: {e}")))?;

        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .map_err(|e| AppError::InvalidPath(format!("could not derive encryption key: {e}")))?;
        Ok(key)
    }

    /// Encrypts `plaintext` with `key`, returning a self-describing
    /// `"enc:v1:" + base64(nonce || ciphertext)` string.
    pub fn encrypt_text(key: &[u8; 32], plaintext: &str) -> Result<String> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Aes256Gcm::generate_nonce(&mut AesOsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| AppError::InvalidPath(format!("could not encrypt value: {e}")))?;
        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        Ok(format!("{ENCRYPTED_PREFIX}{}", BASE64.encode(payload)))
    }

    /// Decrypts a value produced by [`Self::encrypt_text`]. A value
    /// without the `enc:v1:` marker is returned unchanged — plaintext
    /// reads the same whether encryption is on or off.
    pub fn decrypt_text(key: &[u8; 32], stored: &str) -> Result<String> {
        let Some(encoded) = stored.strip_prefix(ENCRYPTED_PREFIX) else {
            return Ok(stored.to_string());
        };
        let payload = BASE64
            .decode(encoded)
            .map_err(|e| AppError::InvalidPath(format!("corrupt encrypted value: {e}")))?;
        if payload.len() < 12 {
            return Err(AppError::InvalidPath(
                "corrupt encrypted value: too short".to_string(),
            ));
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            AppError::InvalidPath("could not decrypt value (wrong key?)".to_string())
        })?;
        String::from_utf8(plaintext)
            .map_err(|e| AppError::InvalidPath(format!("decrypted value is not valid UTF-8: {e}")))
    }

    /// Verifies `password` against the profile-password hash, derives a
    /// fresh key, re-encrypts every existing `user_state.notes`/
    /// `private_tags` value, and marks encryption enabled. Requires a
    /// profile password to already be set — there's no separate
    /// encryption password.
    pub fn enable_metadata_encryption(ctx: &AppContext, password: &str) -> Result<[u8; 32]> {
        if !Self::verify_password(ctx, password)? {
            return Err(AppError::InvalidPath("incorrect password".to_string()));
        }
        // A fresh salt on every enable, so re-enabling after a disable
        // never reuses an old key.
        SettingsService::clear_raw(ctx, METADATA_ENCRYPTION_SALT_KEY)?;
        let key = Self::derive_key(ctx, password)?;
        Self::reencrypt_all(ctx, |value| Self::encrypt_text(&key, value).ok())?;
        SettingsService::set_raw(ctx, METADATA_ENCRYPTION_ENABLED_KEY, "true")?;
        Ok(key)
    }

    /// Verifies `password`, decrypts every `user_state.notes`/
    /// `private_tags` value back to plaintext, and clears the
    /// encryption flag and salt.
    pub fn disable_metadata_encryption(ctx: &AppContext, password: &str) -> Result<()> {
        if !Self::verify_password(ctx, password)? {
            return Err(AppError::InvalidPath("incorrect password".to_string()));
        }
        let key = Self::derive_key(ctx, password)?;
        Self::reencrypt_all(ctx, |value| Self::decrypt_text(&key, value).ok())?;
        SettingsService::set_raw(ctx, METADATA_ENCRYPTION_ENABLED_KEY, "false")?;
        SettingsService::clear_raw(ctx, METADATA_ENCRYPTION_SALT_KEY)?;
        Ok(())
    }

    /// Rewrites every `notes`/`private_tags` value in `user_state`
    /// through `transform` (encrypt or decrypt). Falls back to the
    /// original value if `transform` fails, so a corrupt row is left
    /// alone rather than losing data.
    fn reencrypt_all(ctx: &AppContext, transform: impl Fn(&str) -> Option<String>) -> Result<()> {
        let rows: Vec<(String, Option<String>, Option<String>)> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare("SELECT item_id, notes, private_tags FROM user_state")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?))
                })
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(database::DatabaseError::from)?);
            }
            out
        };

        for (item_id, notes, private_tags) in rows {
            let new_notes = notes.as_deref().and_then(&transform).or(notes);
            let new_tags = private_tags
                .as_deref()
                .and_then(&transform)
                .or(private_tags);
            ctx.db
                .connection()
                .execute(
                    "UPDATE user_state SET notes = ?1, private_tags = ?2 WHERE item_id = ?3",
                    rusqlite::params![new_notes, new_tags, item_id],
                )
                .map_err(database::DatabaseError::from)?;
        }
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

    #[test]
    fn derive_key_is_deterministic_for_the_same_password_and_salt() {
        let (ctx, _dir) = test_ctx();
        let key1 = PrivacyService::derive_key(&ctx, "hunter2").unwrap();
        let key2 = PrivacyService::derive_key(&ctx, "hunter2").unwrap();
        assert_eq!(
            key1, key2,
            "the salt is persisted, so re-deriving must match"
        );
    }

    #[test]
    fn derive_key_differs_for_different_passwords() {
        let (ctx, _dir) = test_ctx();
        let key1 = PrivacyService::derive_key(&ctx, "hunter2").unwrap();
        let key2 = PrivacyService::derive_key(&ctx, "different").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn encrypt_decrypt_round_trips() {
        let key = [7u8; 32];
        let encrypted = PrivacyService::encrypt_text(&key, "a secret note").unwrap();
        assert!(encrypted.starts_with("enc:v1:"));
        let decrypted = PrivacyService::decrypt_text(&key, &encrypted).unwrap();
        assert_eq!(decrypted, "a secret note");
    }

    #[test]
    fn decrypt_text_passes_plaintext_through_unchanged() {
        let key = [7u8; 32];
        assert_eq!(
            PrivacyService::decrypt_text(&key, "plain text").unwrap(),
            "plain text"
        );
    }

    #[test]
    fn decrypt_text_fails_with_the_wrong_key() {
        let key = [7u8; 32];
        let wrong_key = [9u8; 32];
        let encrypted = PrivacyService::encrypt_text(&key, "secret").unwrap();
        assert!(PrivacyService::decrypt_text(&wrong_key, &encrypted).is_err());
    }

    #[test]
    fn enable_metadata_encryption_requires_the_correct_password() {
        let (ctx, _dir) = test_ctx();
        PrivacyService::set_password(&ctx, "hunter2").unwrap();
        let err = PrivacyService::enable_metadata_encryption(&ctx, "wrong").unwrap_err();
        assert!(matches!(err, AppError::InvalidPath(_)));
        assert!(!PrivacyService::metadata_encryption_enabled(&ctx).unwrap());
    }

    #[test]
    fn enable_re_encrypts_existing_notes_and_disable_decrypts_them_back() {
        let (ctx, _dir) = test_ctx();
        let item_id = domain::ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Item', 'unrated', datetime('now'), datetime('now'))",
                rusqlite::params![item_id.to_string()],
            )
            .unwrap();
        crate::user_state::UserStateService::set_notes(&ctx, item_id, Some("my private note"))
            .unwrap();
        PrivacyService::set_password(&ctx, "hunter2").unwrap();

        let key = PrivacyService::enable_metadata_encryption(&ctx, "hunter2").unwrap();
        assert!(PrivacyService::metadata_encryption_enabled(&ctx).unwrap());

        let stored: String = ctx
            .db
            .connection()
            .query_row(
                "SELECT notes FROM user_state WHERE item_id = ?1",
                rusqlite::params![item_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            stored.starts_with("enc:v1:"),
            "notes must be stored encrypted"
        );
        assert_eq!(
            PrivacyService::decrypt_text(&key, &stored).unwrap(),
            "my private note"
        );

        PrivacyService::disable_metadata_encryption(&ctx, "hunter2").unwrap();
        assert!(!PrivacyService::metadata_encryption_enabled(&ctx).unwrap());
        let stored_after: String = ctx
            .db
            .connection()
            .query_row(
                "SELECT notes FROM user_state WHERE item_id = ?1",
                rusqlite::params![item_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored_after, "my private note");
    }
}
