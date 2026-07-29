//! A local, file-backed plugin registry — the "local registry" from
//! `docs/18-plugin-system.md`'s developer-tooling list. This is
//! explicitly **not** the signed, remotely-distributed official
//! registry `docs/48-open-questions-and-decisions.md`'s "Connector
//! distribution" section leaves open: there's no publisher PKI,
//! signature verification, or revocation-list fetching here, because
//! there's no distribution server to fetch one from. What this does
//! provide is the status-lifecycle governance itself (Stable through
//! Removed), independent of where a manifest came from.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

use crate::manifest::PluginManifest;

/// Mirrors the connector "Maintenance policy" vocabulary in
/// `docs/14-source-connectors.md` — a plugin is a sandboxed, 3rd-party
/// connector implementation, so the same status lifecycle applies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Stable,
    Beta,
    Degraded,
    Disabled,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistryEntry {
    pub manifest: PluginManifest,
    pub status: PluginStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse registry file: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("no plugin with id \"{0}\" in the registry")]
    NotFound(String),
    #[error("a plugin with id \"{0}\" is already registered")]
    AlreadyExists(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LocalPluginRegistry {
    entries: Vec<PluginRegistryEntry>,
}

impl LocalPluginRegistry {
    /// Loads the registry from `path`, or returns an empty registry if
    /// the file doesn't exist yet (a fresh install has no plugins).
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), RegistryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn add_entry(
        &mut self,
        manifest: PluginManifest,
        status: PluginStatus,
    ) -> Result<(), RegistryError> {
        if self.entries.iter().any(|e| e.manifest.id == manifest.id) {
            return Err(RegistryError::AlreadyExists(manifest.id));
        }
        self.entries.push(PluginRegistryEntry {
            manifest,
            status,
            added_at: OffsetDateTime::now_utc(),
        });
        Ok(())
    }

    pub fn set_status(&mut self, id: &str, status: PluginStatus) -> Result<(), RegistryError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.manifest.id == id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        entry.status = status;
        Ok(())
    }

    pub fn list(&self) -> &[PluginRegistryEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(id: &str) -> PluginManifest {
        PluginManifest::parse(&format!(
            r#"
id: {id}
name: Example
version: 1.0.0
publisher: Example
api_version: 1
entrypoint: plugin.wasm
capabilities:
  - search
media_types:
  - image
"#
        ))
        .unwrap()
    }

    #[test]
    fn loading_a_missing_file_yields_an_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let registry = LocalPluginRegistry::load(&dir.path().join("registry.json")).unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn add_save_and_reload_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.json");

        let mut registry = LocalPluginRegistry::default();
        registry
            .add_entry(sample_manifest("org.example.connector"), PluginStatus::Beta)
            .unwrap();
        registry.save(&path).unwrap();

        let reloaded = LocalPluginRegistry::load(&path).unwrap();
        assert_eq!(reloaded.list().len(), 1);
        assert_eq!(reloaded.list()[0].manifest.id, "org.example.connector");
        assert_eq!(reloaded.list()[0].status, PluginStatus::Beta);
    }

    #[test]
    fn adding_a_duplicate_id_fails() {
        let mut registry = LocalPluginRegistry::default();
        registry
            .add_entry(sample_manifest("org.example.connector"), PluginStatus::Beta)
            .unwrap();
        let err = registry
            .add_entry(
                sample_manifest("org.example.connector"),
                PluginStatus::Stable,
            )
            .unwrap_err();
        assert!(matches!(err, RegistryError::AlreadyExists(_)));
    }

    #[test]
    fn set_status_transitions_an_existing_entry() {
        let mut registry = LocalPluginRegistry::default();
        registry
            .add_entry(sample_manifest("org.example.connector"), PluginStatus::Beta)
            .unwrap();
        registry
            .set_status("org.example.connector", PluginStatus::Disabled)
            .unwrap();
        assert_eq!(registry.list()[0].status, PluginStatus::Disabled);
    }

    #[test]
    fn set_status_on_an_unknown_id_is_not_found() {
        let mut registry = LocalPluginRegistry::default();
        let err = registry
            .set_status("nonexistent", PluginStatus::Disabled)
            .unwrap_err();
        assert!(matches!(err, RegistryError::NotFound(_)));
    }
}
