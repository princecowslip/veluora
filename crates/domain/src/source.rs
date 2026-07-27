use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{ConnectorId, ItemId, SourceId, SourceRefId};

/// Links a [`crate::media_item::MediaItem`] to a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub id: SourceRefId,
    pub item_id: ItemId,
    pub source_id: SourceId,
    pub source_item_id: String,
    pub canonical_url: Option<String>,
    pub original_title: Option<String>,
    pub original_description: Option<String>,
    pub original_tags: Vec<String>,
    pub access_state: AccessState,
    #[serde(with = "time::serde::rfc3339")]
    pub last_checked_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessState {
    Available,
    Restricted,
    Unavailable,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Unknown,
    Healthy,
    Degraded,
    Unreachable,
}

/// A configured source, backed by a connector (see `docs/14-source-connectors.md`).
///
/// Connectors themselves are out of scope for Milestone A; this type exists
/// so `SourceReference.source_id` and downstream milestones have somewhere
/// to point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub connector_id: ConnectorId,
    pub display_name: String,
    pub enabled: bool,
    pub configuration_json: serde_json::Value,
    /// Opaque reference into the OS credential store — never a raw secret.
    pub credential_ref: Option<String>,
    pub health_state: HealthState,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_health_check: Option<OffsetDateTime>,
    pub capability_snapshot_json: Option<serde_json::Value>,
}
