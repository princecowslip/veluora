//! Veloura domain types: pure entities and logic, no I/O.
//!
//! Mirrors `docs/13-data-model.md`. This crate is deliberately free of
//! database, network, or filesystem dependencies so it can be shared,
//! unmodified, by the local API, CLI, GUI, and application services
//! (ADR-002 in `docs/26-architecture-decisions.md`).

pub mod block_rule;
pub mod collection;
pub mod download;
pub mod ids;
pub mod media_item;
pub mod source;
pub mod submodels;
pub mod tag;
pub mod user_state;
pub mod variant;

pub use block_rule::{BlockCandidate, BlockRule, RuleType, Scope};
pub use collection::{Collection, CollectionType};
pub use download::{ChecksumState, Download, DownloadState};
pub use ids::*;
pub use media_item::{MediaItem, MediaType, RatingClassification, SafetyStatus, VisibilityState};
pub use source::{AccessState, HealthState, Source, SourceReference};
pub use submodels::{Chapter, Gallery, Series, StoryDocument, StoryFormat};
pub use tag::{Tag, TagNamespace};
pub use user_state::{Progress, UserState};
pub use variant::MediaVariant;
