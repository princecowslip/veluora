//! Maps a `Source`'s `connector_id` to the shared connector instance
//! that backs it. Owned by `application::source::SourceService`,
//! which seeds it with this crate's connectors plus its own
//! `LocalFilesystemConnector` (kept in `application` to avoid a
//! dependency cycle — see the crate doc comment).

use std::collections::HashMap;
use std::sync::Arc;

use domain::ConnectorId;

use crate::Connector;

#[derive(Default)]
pub struct ConnectorRegistry {
    connectors: HashMap<ConnectorId, Arc<dyn Connector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, id: ConnectorId, connector: Arc<dyn Connector>) {
        self.connectors.insert(id, connector);
    }

    pub fn get(&self, id: ConnectorId) -> Option<Arc<dyn Connector>> {
        self.connectors.get(&id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{FeedConnector, FEED_CONNECTOR_ID};

    #[test]
    fn register_then_get_round_trips() {
        let mut registry = ConnectorRegistry::new();
        registry.register(FEED_CONNECTOR_ID, Arc::new(FeedConnector::new()));
        assert!(registry.get(FEED_CONNECTOR_ID).is_some());
        assert!(registry.get(ConnectorId::new()).is_none());
    }
}
