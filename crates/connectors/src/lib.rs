//! The `Connector` trait and built-in network connectors —
//! `docs/14-source-connectors.md`. Depends only on `domain` (pure
//! DTOs); the local-filesystem connector lives in `application`
//! instead, since it needs `application`'s services and depending on
//! `application` from here would create a cycle (`application` needs
//! this crate's `Connector` trait/registry to dispatch through).

pub mod booru;
pub mod feed;
mod http_util;
pub mod opds;
pub mod registry;
pub mod trait_def;

pub use booru::{BooruConnector, BOORU_CONNECTOR_ID};
pub use feed::{FeedConnector, FEED_CONNECTOR_ID};
pub use opds::{OpdsConnector, OPDS_CONNECTOR_ID};
pub use registry::ConnectorRegistry;
pub use trait_def::Connector;
