//! The permission model from `docs/18-plugin-system.md`'s "Security
//! model" and "Manifest" sections. Every category is `Option` and
//! defaults to `None` on an omitted manifest block — the default-deny
//! posture is expressed directly in the type, not just enforced by
//! runtime checks elsewhere.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginPermissions {
    #[serde(default)]
    pub network: Option<NetworkPermission>,
    #[serde(default)]
    pub credentials: Option<CredentialPermission>,
    #[serde(default)]
    pub filesystem: Option<FilesystemPermission>,
    #[serde(default)]
    pub local_api: Option<LocalApiPermission>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkPermission {
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialPermission {
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilesystemPermission {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalApiPermission {
    pub scopes: Vec<LocalApiScope>,
}

/// A `docs/19-local-api.md` authorization scope a plugin could
/// request. **Not enforced anywhere yet**: `local-api` issues a single
/// all-or-nothing bearer token today (see `crates/local-api/src/lib.rs`'s
/// `require_auth` middleware) with no concept of scoped tokens. This
/// type exists so the permission model round-trips and can be shown to
/// a user for review — see `docs/18`'s "Permissions UI" — ahead of
/// `local-api` actually growing scoped-token support.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalApiScope {
    ReadLibrary,
    Playback,
    Collections,
    Privacy,
}

impl PluginPermissions {
    pub fn is_empty(&self) -> bool {
        self.network.is_none()
            && self.credentials.is_none()
            && self.filesystem.is_none()
            && self.local_api.is_none()
    }

    /// The human-readable review list `docs/18`'s "Permissions UI"
    /// section asks be shown before installation: requested domains,
    /// credential scope, filesystem access, local-API scope. Consumed
    /// by the `plugin validate` CLI command — there's no GUI/TUI
    /// install flow to show this in yet, since there's no real plugin
    /// to install.
    pub fn summary_lines(&self) -> Vec<String> {
        if self.is_empty() {
            return vec!["No permissions requested.".to_string()];
        }

        let mut lines = Vec::new();
        if let Some(network) = &self.network {
            lines.push(format!("Network: {}", network.domains.join(", ")));
        }
        if let Some(credentials) = &self.credentials {
            lines.push(format!("Credentials: {}", credentials.scopes.join(", ")));
        }
        if let Some(filesystem) = &self.filesystem {
            if !filesystem.read.is_empty() {
                lines.push(format!("Filesystem read: {}", filesystem.read.join(", ")));
            }
            if !filesystem.write.is_empty() {
                lines.push(format!("Filesystem write: {}", filesystem.write.join(", ")));
            }
        }
        if let Some(local_api) = &self.local_api {
            let scopes: Vec<&str> = local_api
                .scopes
                .iter()
                .map(|s| match s {
                    LocalApiScope::ReadLibrary => "read_library",
                    LocalApiScope::Playback => "playback",
                    LocalApiScope::Collections => "collections",
                    LocalApiScope::Privacy => "privacy",
                })
                .collect();
            lines.push(format!(
                "Local API (not yet enforced by local-api): {}",
                scopes.join(", ")
            ));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_permissions_are_empty_and_summarized_as_none_requested() {
        let permissions = PluginPermissions::default();
        assert!(permissions.is_empty());
        assert_eq!(
            permissions.summary_lines(),
            vec!["No permissions requested.".to_string()]
        );
    }

    #[test]
    fn summarizes_every_requested_category() {
        let permissions = PluginPermissions {
            network: Some(NetworkPermission {
                domains: vec!["api.example.test".to_string()],
            }),
            credentials: Some(CredentialPermission {
                scopes: vec!["source".to_string()],
            }),
            filesystem: Some(FilesystemPermission {
                read: vec!["/cache".to_string()],
                write: vec![],
            }),
            local_api: Some(LocalApiPermission {
                scopes: vec![LocalApiScope::ReadLibrary],
            }),
        };
        assert!(!permissions.is_empty());
        let lines = permissions.summary_lines();
        assert!(lines.iter().any(|l| l.contains("api.example.test")));
        assert!(lines.iter().any(|l| l.contains("source")));
        assert!(lines.iter().any(|l| l.contains("/cache")));
        assert!(lines.iter().any(|l| l.contains("read_library")));
    }
}
