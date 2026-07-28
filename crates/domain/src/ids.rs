//! Newtype identifiers for domain entities.
//!
//! Wrapping `Uuid` per entity prevents accidentally passing, say, a
//! `SourceId` where an `ItemId` is expected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_type!(ItemId);
id_type!(SourceRefId);
id_type!(VariantId);
id_type!(CollectionId);
id_type!(TagId);
id_type!(CreatorId);
id_type!(SeriesId);
id_type!(ChapterId);
id_type!(BlockRuleId);
id_type!(DownloadId);
id_type!(SourceId);
id_type!(ConnectorId);
id_type!(LibraryRootId);
