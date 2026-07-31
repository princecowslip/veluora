//! Profile-password locking and local data/cache management for the
//! Privacy Center screen.
//!
//! Password storage uses `argon2` (the PHC-recommended password hashing
//! function) rather than any custom scheme, per
//! `docs/20-privacy-and-security.md`'s "do not invent custom
//! cryptography for password storage" rule.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

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

/// Per-subdirectory byte totals under `<data_dir>/cache`, for the Privacy
/// Center's cache display.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheBreakdown {
    pub thumbnails_bytes: u64,
    pub stories_bytes: u64,
    pub other_bytes: u64,
    pub total_bytes: u64,
}

/// What [`PrivacyService::enforce_cache_quota`] did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEvictionReport {
    pub evicted_files: u64,
    pub evicted_bytes: u64,
    pub remaining_bytes: u64,
}

/// A file eligible for LRU-style eviction under a byte quota — the
/// common shape [`PrivacyService::enforce_cache_quota`] and
/// [`PrivacyService::enforce_download_quota`] both reduce to, so the
/// actual "sort by recency, skip protected, evict oldest-first until
/// under quota" loop is written once.
struct EvictionCandidate {
    path: PathBuf,
    recency: std::time::SystemTime,
    size: u64,
    protected: bool,
}

fn no_op_report(remaining_bytes: u64) -> CacheEvictionReport {
    CacheEvictionReport {
        evicted_files: 0,
        evicted_bytes: 0,
        remaining_bytes,
    }
}

fn evict_until_under_quota(
    mut remaining: u64,
    quota: u64,
    mut candidates: Vec<EvictionCandidate>,
) -> CacheEvictionReport {
    candidates.retain(|c| !c.protected);
    candidates.sort_by_key(|c| c.recency);

    let mut evicted_files = 0u64;
    let mut evicted_bytes = 0u64;
    for candidate in candidates {
        if remaining <= quota {
            break;
        }
        if fs::remove_file(&candidate.path).is_ok() {
            remaining = remaining.saturating_sub(candidate.size);
            evicted_files += 1;
            evicted_bytes += candidate.size;
        }
    }

    CacheEvictionReport {
        evicted_files,
        evicted_bytes,
        remaining_bytes: remaining,
    }
}

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

    /// Breaks [`Self::cache_size_bytes`]'s total down by the top-level
    /// subdirectory each file lives under (`thumbnails`, `stories`, or
    /// `other` for anything else).
    pub fn cache_breakdown(ctx: &AppContext) -> Result<CacheBreakdown> {
        let cache_dir = ctx.data_dir.join("cache");
        let mut breakdown = CacheBreakdown {
            thumbnails_bytes: 0,
            stories_bytes: 0,
            other_bytes: 0,
            total_bytes: 0,
        };
        if !cache_dir.exists() {
            return Ok(breakdown);
        }
        for entry in walkdir::WalkDir::new(&cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let top_level = entry
                .path()
                .strip_prefix(&cache_dir)
                .ok()
                .and_then(|p| p.components().next())
                .map(|c| c.as_os_str().to_string_lossy().into_owned());
            match top_level.as_deref() {
                Some("thumbnails") => breakdown.thumbnails_bytes += size,
                Some("stories") => breakdown.stories_bytes += size,
                _ => breakdown.other_bytes += size,
            }
            breakdown.total_bytes += size;
        }
        Ok(breakdown)
    }

    /// If a quota is set (via [`SettingsService::cache_quota_bytes`]) and
    /// the cache exceeds it, deletes thumbnail files oldest-mtime-first
    /// (LRU) — excluding any variant belonging to a pinned item — until
    /// under quota or no evictable files remain. An explicit action, not
    /// run automatically: matches how backup/restore and encryption stay
    /// explicit user actions elsewhere in this service.
    pub fn enforce_cache_quota(ctx: &AppContext) -> Result<CacheEvictionReport> {
        let remaining = Self::cache_size_bytes(ctx)?;
        let Some(quota) = SettingsService::cache_quota_bytes(ctx)? else {
            return Ok(no_op_report(remaining));
        };
        if remaining <= quota {
            return Ok(no_op_report(remaining));
        }

        let pinned_variant_ids: HashSet<String> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare(
                    "SELECT mv.id FROM media_variants mv \
                     JOIN user_state us ON us.item_id = mv.item_id \
                     WHERE us.pinned = 1",
                )
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(database::DatabaseError::from)?;
            let mut set = HashSet::new();
            for row in rows {
                set.insert(row.map_err(database::DatabaseError::from)?);
            }
            set
        };

        let thumbnails_dir = ctx.data_dir.join("cache").join("thumbnails");
        let candidates: Vec<EvictionCandidate> = walkdir::WalkDir::new(&thumbnails_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|entry| {
                let variant_id = entry.path().file_stem()?.to_str()?.to_string();
                let metadata = entry.metadata().ok()?;
                Some(EvictionCandidate {
                    path: entry.path().to_path_buf(),
                    recency: metadata.modified().ok()?,
                    size: metadata.len(),
                    protected: pinned_variant_ids.contains(&variant_id),
                })
            })
            .collect();

        Ok(evict_until_under_quota(remaining, quota, candidates))
    }

    /// Sums the file sizes of every `Completed` download's permanent
    /// file — the download-directory counterpart of
    /// [`Self::cache_size_bytes`].
    pub fn download_directory_size_bytes(ctx: &AppContext) -> Result<u64> {
        let conn = ctx.db.connection();
        let paths: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT destination FROM downloads WHERE state = 'completed'")
                .map_err(database::DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(database::DatabaseError::from)?);
            }
            out
        };
        let mut total = 0u64;
        for path in paths {
            total += fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        }
        Ok(total)
    }

    /// If a quota is set (via [`SettingsService::download_quota_bytes`])
    /// and completed downloads exceed it, deletes permanent download
    /// files oldest-`completed_at`-first — excluding any download
    /// marked `pinned` on its own row *or* whose owning item has
    /// `user_state.pinned` set (so pinning an item anywhere also
    /// protects its downloads) — until under quota. Marks each evicted
    /// row `Evicted` and clears its variant's `local_path`/`file_size`/
    /// `checksum`, so the item stays in the library, now remote-only
    /// again. `Active`/`Paused`/`Queued`/`Failed` rows are never
    /// touched. An explicit action, never run automatically.
    pub fn enforce_download_quota(ctx: &AppContext) -> Result<CacheEvictionReport> {
        let remaining = Self::download_directory_size_bytes(ctx)?;
        let Some(quota) = SettingsService::download_quota_bytes(ctx)? else {
            return Ok(no_op_report(remaining));
        };
        if remaining <= quota {
            return Ok(no_op_report(remaining));
        }

        struct DownloadRow {
            id: String,
            destination: String,
            completed_at: Option<String>,
            pinned: bool,
            item_pinned: bool,
        }
        let rows: Vec<DownloadRow> = {
            let conn = ctx.db.connection();
            let mut stmt = conn
                .prepare(
                    "SELECT d.id, d.destination, d.completed_at, d.pinned, \
                     COALESCE(us.pinned, 0) \
                     FROM downloads d \
                     LEFT JOIN user_state us ON us.item_id = d.item_id \
                     WHERE d.state = 'completed'",
                )
                .map_err(database::DatabaseError::from)?;
            let mapped = stmt
                .query_map([], |row| {
                    Ok(DownloadRow {
                        id: row.get(0)?,
                        destination: row.get(1)?,
                        completed_at: row.get(2)?,
                        pinned: row.get::<_, i64>(3)? != 0,
                        item_pinned: row.get::<_, i64>(4)? != 0,
                    })
                })
                .map_err(database::DatabaseError::from)?;
            let mut out = Vec::new();
            for row in mapped {
                out.push(row.map_err(database::DatabaseError::from)?);
            }
            out
        };

        let candidates: Vec<EvictionCandidate> = rows
            .iter()
            .map(|row| {
                let path = PathBuf::from(&row.destination);
                let recency = row
                    .completed_at
                    .as_deref()
                    .and_then(crate::time_format::from_rfc3339)
                    .map(std::time::SystemTime::from)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                EvictionCandidate {
                    path,
                    recency,
                    size,
                    protected: row.pinned || row.item_pinned,
                }
            })
            .collect();

        let report = evict_until_under_quota(remaining, quota, candidates);

        // Mark whichever files actually got removed as `Evicted` and
        // clear their variant's local-file fields. `evict_until_under_quota`
        // only reports counts, not which paths it removed, so this
        // reconciles by checking existence after the fact — the same
        // approach `enforce_cache_quota` doesn't need (nothing there
        // tracks per-file DB rows).
        let conn = ctx.db.connection();
        for row in &rows {
            if !PathBuf::from(&row.destination).exists() {
                conn.execute(
                    "UPDATE downloads SET state = 'evicted', temp_path = NULL WHERE id = ?1",
                    rusqlite::params![row.id],
                )
                .map_err(database::DatabaseError::from)?;
                conn.execute(
                    "UPDATE media_variants SET local_path = NULL, file_size = NULL, checksum = NULL \
                     WHERE local_path = ?1",
                    rusqlite::params![row.destination],
                )
                .map_err(database::DatabaseError::from)?;
            }
        }

        Ok(report)
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
    fn cache_breakdown_splits_bytes_by_top_level_subdirectory() {
        let (ctx, _dir) = test_ctx();
        let cache_dir = ctx.data_dir.join("cache");
        fs::create_dir_all(cache_dir.join("thumbnails")).unwrap();
        fs::create_dir_all(cache_dir.join("stories")).unwrap();
        fs::write(cache_dir.join("thumbnails").join("a.jpg"), vec![0u8; 100]).unwrap();
        fs::write(cache_dir.join("stories").join("b.txt"), vec![0u8; 50]).unwrap();
        fs::write(cache_dir.join("loose.bin"), vec![0u8; 10]).unwrap();

        let breakdown = PrivacyService::cache_breakdown(&ctx).unwrap();
        assert_eq!(breakdown.thumbnails_bytes, 100);
        assert_eq!(breakdown.stories_bytes, 50);
        assert_eq!(breakdown.other_bytes, 10);
        assert_eq!(breakdown.total_bytes, 160);
    }

    fn insert_item_with_variant(ctx: &AppContext) -> (domain::ItemId, domain::VariantId) {
        let item_id = domain::ItemId::new();
        let variant_id = domain::VariantId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Item', 'unrated', datetime('now'), datetime('now'))",
                rusqlite::params![item_id.to_string()],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_variants (id, item_id, mime_type, format) VALUES (?1, ?2, 'image/jpeg', 'jpeg')",
                rusqlite::params![variant_id.to_string(), item_id.to_string()],
            )
            .unwrap();
        (item_id, variant_id)
    }

    #[test]
    fn enforce_cache_quota_is_a_no_op_when_no_quota_is_set() {
        let (ctx, _dir) = test_ctx();
        let (_item_id, variant_id) = insert_item_with_variant(&ctx);
        let thumb_dir = ctx.data_dir.join("cache").join("thumbnails");
        fs::create_dir_all(&thumb_dir).unwrap();
        fs::write(thumb_dir.join(format!("{variant_id}.jpg")), vec![0u8; 1024]).unwrap();

        let report = PrivacyService::enforce_cache_quota(&ctx).unwrap();
        assert_eq!(report.evicted_files, 0);
        assert_eq!(report.evicted_bytes, 0);
        assert_eq!(report.remaining_bytes, 1024);
        assert_eq!(PrivacyService::cache_size_bytes(&ctx).unwrap(), 1024);
    }

    #[test]
    fn enforce_cache_quota_evicts_oldest_first_and_never_evicts_pinned() {
        let (ctx, _dir) = test_ctx();
        let (_old_item, old_variant) = insert_item_with_variant(&ctx);
        let (pinned_item, pinned_variant) = insert_item_with_variant(&ctx);
        let (_new_item, new_variant) = insert_item_with_variant(&ctx);
        crate::user_state::UserStateService::set_pinned(&ctx, pinned_item, true).unwrap();

        let thumb_dir = ctx.data_dir.join("cache").join("thumbnails");
        fs::create_dir_all(&thumb_dir).unwrap();

        let write_with_mtime = |variant_id: domain::VariantId, seconds_ago: u64| {
            let path = thumb_dir.join(format!("{variant_id}.jpg"));
            fs::write(&path, vec![0u8; 1000]).unwrap();
            let mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds_ago);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(mtime)
                .unwrap();
            path
        };

        let old_path = write_with_mtime(old_variant, 300);
        let pinned_path = write_with_mtime(pinned_variant, 200);
        let new_path = write_with_mtime(new_variant, 10);

        SettingsService::set_cache_quota_bytes(&ctx, Some(2500)).unwrap();
        let report = PrivacyService::enforce_cache_quota(&ctx).unwrap();

        assert!(!old_path.exists(), "oldest non-pinned file must be evicted");
        assert!(pinned_path.exists(), "pinned file must never be evicted");
        assert!(new_path.exists(), "newest file has no need to be evicted");
        assert_eq!(report.evicted_files, 1);
        assert_eq!(report.evicted_bytes, 1000);
        assert_eq!(report.remaining_bytes, 2000);
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_completed_download(
        ctx: &AppContext,
        item_id: domain::ItemId,
        variant_id: domain::VariantId,
        destination: &std::path::Path,
        bytes: usize,
        seconds_ago: u64,
        pinned: bool,
    ) -> String {
        fs::write(destination, vec![0u8; bytes]).unwrap();
        let download_id = domain::DownloadId::new().to_string();
        let completed_at = to_rfc3339(
            time::OffsetDateTime::now_utc() - std::time::Duration::from_secs(seconds_ago),
        );
        ctx.db
            .connection()
            .execute(
                "INSERT INTO downloads (id, item_id, variant_id, state, destination, bytes_received, \
                 checksum_state, retry_count, created_at, completed_at, pinned) \
                 VALUES (?1, ?2, ?3, 'completed', ?4, ?5, 'unavailable', 0, datetime('now'), ?6, ?7)",
                rusqlite::params![
                    download_id,
                    item_id.to_string(),
                    variant_id.to_string(),
                    destination.to_string_lossy(),
                    bytes as i64,
                    completed_at,
                    pinned as i64,
                ],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "UPDATE media_variants SET local_path = ?2 WHERE id = ?1",
                rusqlite::params![variant_id.to_string(), destination.to_string_lossy()],
            )
            .unwrap();
        download_id
    }

    #[test]
    fn enforce_download_quota_is_a_no_op_when_no_quota_is_set() {
        let (ctx, _dir) = test_ctx();
        let (item_id, variant_id) = insert_item_with_variant(&ctx);
        let downloads_dir = ctx.data_dir.join("downloads");
        fs::create_dir_all(&downloads_dir).unwrap();
        insert_completed_download(
            &ctx,
            item_id,
            variant_id,
            &downloads_dir.join("a.bin"),
            1024,
            10,
            false,
        );

        let report = PrivacyService::enforce_download_quota(&ctx).unwrap();
        assert_eq!(report.evicted_files, 0);
        assert_eq!(report.remaining_bytes, 1024);
    }

    #[test]
    fn enforce_download_quota_evicts_oldest_first_and_never_evicts_pinned_downloads_or_pinned_items(
    ) {
        let (ctx, _dir) = test_ctx();
        let downloads_dir = ctx.data_dir.join("downloads");
        fs::create_dir_all(&downloads_dir).unwrap();

        let (old_item, old_variant) = insert_item_with_variant(&ctx);
        let (row_pinned_item, row_pinned_variant) = insert_item_with_variant(&ctx);
        let (item_pinned_item, item_pinned_variant) = insert_item_with_variant(&ctx);
        let (_new_item, new_variant) = insert_item_with_variant(&ctx);
        crate::user_state::UserStateService::set_pinned(&ctx, item_pinned_item, true).unwrap();

        let old_path = downloads_dir.join("old.bin");
        let row_pinned_path = downloads_dir.join("row-pinned.bin");
        let item_pinned_path = downloads_dir.join("item-pinned.bin");
        let new_path = downloads_dir.join("new.bin");

        insert_completed_download(&ctx, old_item, old_variant, &old_path, 1000, 400, false);
        insert_completed_download(
            &ctx,
            row_pinned_item,
            row_pinned_variant,
            &row_pinned_path,
            1000,
            300,
            true,
        );
        insert_completed_download(
            &ctx,
            item_pinned_item,
            item_pinned_variant,
            &item_pinned_path,
            1000,
            200,
            false,
        );
        insert_completed_download(
            &ctx,
            item_pinned_item,
            new_variant,
            &new_path,
            1000,
            10,
            false,
        );

        SettingsService::set_download_quota_bytes(&ctx, Some(2500)).unwrap();
        let report = PrivacyService::enforce_download_quota(&ctx).unwrap();

        assert!(
            !old_path.exists(),
            "oldest non-pinned download must be evicted"
        );
        assert!(
            row_pinned_path.exists(),
            "row-pinned download must never be evicted"
        );
        assert!(
            item_pinned_path.exists(),
            "download of a pinned item must never be evicted"
        );
        assert!(
            new_path.exists(),
            "newest download has no need to be evicted"
        );
        assert_eq!(report.evicted_files, 1);
        assert_eq!(report.evicted_bytes, 1000);

        let variant_local_path: Option<String> = ctx
            .db
            .connection()
            .query_row(
                "SELECT local_path FROM media_variants WHERE id = ?1",
                rusqlite::params![old_variant.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            variant_local_path.is_none(),
            "evicted variant must go remote-only again"
        );
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
