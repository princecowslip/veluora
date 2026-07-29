//! Plugin manifest parsing and validation — the YAML shape from
//! `docs/18-plugin-system.md`'s "Manifest" section, parsed and checked
//! independently of whether any real plugin runtime exists to load the
//! `entrypoint` yet.

use std::path::PathBuf;

use domain::MediaType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::permissions::PluginPermissions;

/// The API versions this build of the plugin host understands. Only
/// `1` exists today — a placeholder for the compatibility-range
/// negotiation `docs/18`'s package-signing section describes, which
/// needs a real registry server to be meaningful.
const SUPPORTED_API_VERSIONS: &[u32] = &[1];

/// Capability strings a manifest may declare, per `docs/14-source-connectors.md`'s
/// capability model — reused here since a plugin *is* a (sandboxed,
/// third-party) connector implementation.
const KNOWN_CAPABILITIES: &[&str] = &["search", "browse", "item_details", "streaming"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub api_version: u32,
    pub entrypoint: PathBuf,
    #[serde(default)]
    pub permissions: PluginPermissions,
    pub capabilities: Vec<String>,
    pub media_types: Vec<MediaType>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("could not parse manifest YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("could not read manifest file: {0}")]
    Io(#[from] std::io::Error),
}

/// One thing wrong with a manifest, surfaced from [`PluginManifest::validate`]
/// rather than a hard parse error — a manifest can parse successfully
/// (it's well-formed YAML matching the shape) while still being invalid
/// to install (e.g. an unsupported `api_version`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

impl PluginManifest {
    pub fn parse(yaml: &str) -> Result<Self, ManifestError> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    pub fn load(path: &std::path::Path) -> Result<Self, ManifestError> {
        let contents = std::fs::read_to_string(path)?;
        Self::parse(&contents)
    }

    /// Checks the manifest for problems that parsing alone can't catch.
    /// Returns every issue found, not just the first — so a manifest
    /// author (or the `plugin validate` CLI command) sees the whole
    /// list in one pass.
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if !self.id.contains('.') {
            issues.push(ValidationIssue {
                field: "id".to_string(),
                message: "should be reverse-DNS-shaped, e.g. \"org.example.connector\"".to_string(),
            });
        }

        if !SUPPORTED_API_VERSIONS.contains(&self.api_version) {
            issues.push(ValidationIssue {
                field: "api_version".to_string(),
                message: format!(
                    "unsupported api_version {} (supported: {:?})",
                    self.api_version, SUPPORTED_API_VERSIONS
                ),
            });
        }

        if self.capabilities.is_empty() {
            issues.push(ValidationIssue {
                field: "capabilities".to_string(),
                message: "must declare at least one capability".to_string(),
            });
        }
        for capability in &self.capabilities {
            if !KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                issues.push(ValidationIssue {
                    field: "capabilities".to_string(),
                    message: format!(
                        "unknown capability \"{capability}\" (known: {KNOWN_CAPABILITIES:?})"
                    ),
                });
            }
        }

        if let Some(network) = &self.permissions.network {
            if network.domains.is_empty() {
                issues.push(ValidationIssue {
                    field: "permissions.network.domains".to_string(),
                    message: "a network permission block must list at least one domain".to_string(),
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_yaml() -> &'static str {
        r#"
id: org.example.connector
name: Example Source
version: 1.2.0
publisher: Example
api_version: 1
entrypoint: plugin.wasm
permissions:
  network:
    domains:
      - api.example.test
  credentials:
    scopes:
      - source
capabilities:
  - search
  - browse
media_types:
  - image
"#
    }

    #[test]
    fn parses_the_docs_18_example_manifest() {
        let manifest = PluginManifest::parse(valid_yaml()).unwrap();
        assert_eq!(manifest.id, "org.example.connector");
        assert_eq!(manifest.media_types, vec![MediaType::Image]);
        assert_eq!(
            manifest.permissions.network.as_ref().unwrap().domains,
            vec!["api.example.test".to_string()]
        );
    }

    #[test]
    fn a_valid_manifest_has_no_validation_issues() {
        let manifest = PluginManifest::parse(valid_yaml()).unwrap();
        assert!(manifest.validate().is_empty());
    }

    #[test]
    fn flags_a_non_reverse_dns_id() {
        let yaml = valid_yaml().replace("org.example.connector", "example");
        let manifest = PluginManifest::parse(&yaml).unwrap();
        let issues = manifest.validate();
        assert!(issues.iter().any(|i| i.field == "id"));
    }

    #[test]
    fn flags_an_unsupported_api_version() {
        let yaml = valid_yaml().replace("api_version: 1", "api_version: 99");
        let manifest = PluginManifest::parse(&yaml).unwrap();
        let issues = manifest.validate();
        assert!(issues.iter().any(|i| i.field == "api_version"));
    }

    #[test]
    fn flags_empty_capabilities() {
        let yaml =
            valid_yaml().replace("capabilities:\n  - search\n  - browse", "capabilities: []");
        let manifest = PluginManifest::parse(&yaml).unwrap();
        let issues = manifest.validate();
        assert!(issues
            .iter()
            .any(|i| i.field == "capabilities" && i.message.contains("at least one")));
    }

    #[test]
    fn flags_an_unknown_capability() {
        let yaml = valid_yaml().replace("- search", "- teleportation");
        let manifest = PluginManifest::parse(&yaml).unwrap();
        let issues = manifest.validate();
        assert!(issues
            .iter()
            .any(|i| i.field == "capabilities" && i.message.contains("teleportation")));
    }

    #[test]
    fn flags_a_network_permission_with_no_domains() {
        let yaml = valid_yaml().replace(
            "network:\n    domains:\n      - api.example.test",
            "network:\n    domains: []",
        );
        let manifest = PluginManifest::parse(&yaml).unwrap();
        let issues = manifest.validate();
        assert!(issues
            .iter()
            .any(|i| i.field == "permissions.network.domains"));
    }

    #[test]
    fn malformed_yaml_is_a_parse_error() {
        let err = PluginManifest::parse("not: valid: yaml: at all: [").unwrap_err();
        assert!(matches!(err, ManifestError::Parse(_)));
    }
}
