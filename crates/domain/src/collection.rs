use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{CollectionId, ItemId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionType {
    Manual,
    Smart,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub description: Option<String>,
    pub collection_type: CollectionType,
    /// Saved query string, present when `collection_type` is `Smart`.
    pub query: Option<String>,
    pub sort_mode: String,
    pub cover_item_id: Option<ItemId>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
