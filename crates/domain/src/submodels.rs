//! Media-type submodels: [`Gallery`], [`Series`]/[`Chapter`], and
//! [`StoryDocument`], per `docs/13-data-model.md`.

use serde::{Deserialize, Serialize};

use crate::ids::{ChapterId, CreatorId, ItemId, SeriesId, SourceRefId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gallery {
    pub id: ItemId,
    pub parent_item_id: ItemId,
    pub ordered_child_ids: Vec<ItemId>,
    pub cover_child_id: Option<ItemId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: SeriesId,
    pub title: String,
    pub creator_ids: Vec<CreatorId>,
    pub source_ref_ids: Vec<SourceRefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub series_id: SeriesId,
    pub chapter_number: Option<f32>,
    pub volume_number: Option<f32>,
    pub title: String,
    pub ordered_page_ids: Vec<ItemId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryFormat {
    PlainText,
    Markdown,
    Html,
    Epub,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryDocument {
    pub item_id: ItemId,
    pub format: StoryFormat,
    /// Path to sanitized, locally stored content — never rendered from
    /// untrusted raw source markup directly.
    pub sanitized_content_location: String,
    pub chapter_map: serde_json::Value,
    pub text_index_location: Option<String>,
}
