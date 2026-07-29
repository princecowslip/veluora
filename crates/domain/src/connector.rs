//! Connector capability and result types from
//! `docs/14-source-connectors.md`. Pure DTOs only — the `Connector`
//! trait itself performs I/O (HTTP fetches) and so lives in the
//! `connectors` crate, not here (this crate stays I/O-free per
//! ADR-002 in `docs/26-architecture-decisions.md`).

use serde::{Deserialize, Serialize};

use crate::media_item::MediaType;

/// What a connector declares it can do, checked by the application
/// before showing an action — per `docs/14`'s capability model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCapabilities {
    pub search: bool,
    pub browse: bool,
    pub item_details: bool,
    pub streaming: bool,
    pub downloads: bool,
    pub comments: bool,
    pub authentication: Vec<AuthMethod>,
    pub media_types: Vec<MediaType>,
    pub pagination: PaginationMode,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    OAuth,
    ApiToken,
    Cookie,
    SessionToken,
    Anonymous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaginationMode {
    Cursor,
    Offset,
    Page,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests: u32,
    pub period_seconds: u32,
}

/// A connector call's outcome. Deliberately not a plain
/// `Result<T, String>` — `docs/14` is explicit: *"Do not represent all
/// failures as empty results."* A caller can distinguish "nothing
/// found" (`NotFound`) from "the source needs auth" from "we only got
/// some of it" (`Partial`) and react accordingly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum ConnectorResult<T> {
    Success(T),
    Partial(T),
    AuthenticationRequired,
    RateLimited,
    UnsupportedQuery,
    UnsupportedCapability,
    NotFound,
    Deleted,
    BlockedBySource,
    TemporaryFailure(String),
    PermanentFailure(String),
}

/// A connector-native item, before it's imported into the local
/// library. Deliberately smaller than [`crate::media_item::MediaItem`]
/// — a connector can't know everything a fully-scanned local file
/// knows (variants, precise duration, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteItem {
    pub source_item_id: String,
    pub title: String,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub tags: Vec<String>,
    pub media_type: MediaType,
    pub thumbnail_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_result_serializes_as_adjacently_tagged_json() {
        let success = ConnectorResult::Success(42);
        let json = serde_json::to_value(&success).unwrap();
        assert_eq!(json, serde_json::json!({ "status": "success", "data": 42 }));

        let not_found: ConnectorResult<i32> = ConnectorResult::NotFound;
        let json = serde_json::to_value(&not_found).unwrap();
        assert_eq!(json, serde_json::json!({ "status": "not_found" }));

        let temp_failure: ConnectorResult<i32> =
            ConnectorResult::TemporaryFailure("timed out".to_string());
        let json = serde_json::to_value(&temp_failure).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "status": "temporary_failure", "data": "timed out" })
        );
    }
}
