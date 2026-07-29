//! The `Connector` interface from `docs/14-source-connectors.md`.
//!
//! Connectors are stateless, shared singletons (one `FeedConnector`
//! instance backs every feed-configured `Source`) — per-source state
//! (URL, credentials, capability snapshot) lives in the `domain::Source`
//! row itself and is passed into each call, rather than mutated on the
//! connector object. This avoids `&mut self` entirely, so a connector
//! can be safely shared behind `Arc<dyn Connector>` across concurrent
//! callers with no locking.
//!
//! Optional methods default to `ConnectorResult::UnsupportedCapability`
//! — a connector only needs to override what it actually declares in
//! [`Connector::capabilities`].

use async_trait::async_trait;
use domain::{
    ConnectorCapabilities, ConnectorResult, HealthState, RemoteItem, SearchQuery, Source,
};

#[async_trait]
pub trait Connector: Send + Sync {
    /// A short, stable, human-readable name — not the manifest-style
    /// `ConnectorId`, which identifies *which* connector a `Source`
    /// row is backed by.
    fn identify(&self) -> &'static str;

    fn capabilities(&self) -> ConnectorCapabilities;

    async fn configure(&self, _source: &Source, _config: serde_json::Value) -> ConnectorResult<()> {
        ConnectorResult::UnsupportedCapability
    }

    async fn authenticate(&self, _source: &Source) -> ConnectorResult<()> {
        ConnectorResult::UnsupportedCapability
    }

    async fn logout(&self, _source: &Source) -> ConnectorResult<()> {
        ConnectorResult::UnsupportedCapability
    }

    async fn health_check(&self, _source: &Source) -> ConnectorResult<HealthState> {
        ConnectorResult::Success(HealthState::Unknown)
    }

    /// Full query-language search — only meaningful when
    /// `capabilities().search` is true. See `SourceService::browse`
    /// for how unsupported query clauses are backfilled locally.
    async fn search(
        &self,
        _source: &Source,
        _query: &SearchQuery,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        ConnectorResult::UnsupportedCapability
    }

    /// Unfiltered (or lightly paginated) listing — the only retrieval
    /// mode a source with `capabilities().search == false` supports.
    async fn browse(
        &self,
        _source: &Source,
        _page: Option<&str>,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        ConnectorResult::UnsupportedCapability
    }

    async fn get_item(
        &self,
        _source: &Source,
        _source_item_id: &str,
    ) -> ConnectorResult<RemoteItem> {
        ConnectorResult::UnsupportedCapability
    }

    async fn get_gallery(
        &self,
        _source: &Source,
        _source_item_id: &str,
    ) -> ConnectorResult<Vec<RemoteItem>> {
        ConnectorResult::UnsupportedCapability
    }

    async fn resolve_variants(
        &self,
        _source: &Source,
        _source_item_id: &str,
    ) -> ConnectorResult<Vec<String>> {
        ConnectorResult::UnsupportedCapability
    }

    async fn get_tags(&self, _source: &Source, _prefix: &str) -> ConnectorResult<Vec<String>> {
        ConnectorResult::UnsupportedCapability
    }

    async fn refresh_access(&self, _source: &Source) -> ConnectorResult<()> {
        ConnectorResult::UnsupportedCapability
    }
}
