//! The `Connector` trait and built-in network connectors —
//! `docs/14-source-connectors.md`. Depends only on `domain` (pure
//! DTOs); the local-filesystem connector lives in `application`
//! instead, since it needs `application`'s services and depending on
//! `application` from here would create a cycle (`application` needs
//! this crate's `Connector` trait/registry to dispatch through).

pub mod feed;
pub mod registry;
pub mod trait_def;

pub use feed::{FeedConnector, FEED_CONNECTOR_ID};
pub use registry::ConnectorRegistry;
pub use trait_def::Connector;
