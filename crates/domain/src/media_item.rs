use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{ItemId, SeriesId, TagId};

/// The kind of work a [`MediaItem`] represents.
///
/// Mirrors the media types called out across `docs/13-data-model.md` and
/// `docs/04-feature-catalogue.md`. Kept as an open enum with `Other` so a
/// connector can surface a type this milestone hasn't modeled yet without
/// failing deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Video,
    Image,
    Gallery,
    Audio,
    Story,
    Manga,
    Comic,
    Other,
}

/// Source/content rating classification, distinct from a user's personal
/// [`crate::user_state::UserState::rating`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatingClassification {
    Unrated,
    General,
    Mature,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyStatus {
    Unreviewed,
    Approved,
    Flagged,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityState {
    Visible,
    Hidden,
    Blurred,
}

/// One logical work or media entry.
///
/// Field-for-field mirror of the `MediaItem` entity in
/// `docs/13-data-model.md`. Classification fields
/// (`category_ids`, `act_tag_ids`, `genre_tag_ids`, `production_tag_ids`,
/// `visibility_state`, `blur_policy_id`, `visual_orientation`,
/// `sexual_orientation_categories`, `participant_composition`,
/// `gender_identity_categories`) are optional and may come from source
/// metadata, local mapping, or user edits — the domain layer never infers
/// real-person identity or orientation from appearance or scene
/// participation; it only carries values supplied to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: ItemId,
    pub media_type: MediaType,
    pub title: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub rating_classification: RatingClassification,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub discovered_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub creator_ids: Vec<crate::ids::CreatorId>,
    pub series_id: Option<SeriesId>,
    pub source_ref_ids: Vec<crate::ids::SourceRefId>,
    pub variant_ids: Vec<crate::ids::VariantId>,
    pub tag_ids: Vec<TagId>,
    pub category_ids: Vec<TagId>,
    pub act_tag_ids: Vec<TagId>,
    pub genre_tag_ids: Vec<TagId>,
    pub production_tag_ids: Vec<TagId>,
    pub safety_status: SafetyStatus,
    pub visibility_state: VisibilityState,
    pub blur_policy_id: Option<String>,
    pub visual_orientation: Option<String>,
    pub sexual_orientation_categories: Option<Vec<String>>,
    pub participant_composition: Option<Vec<String>>,
    pub gender_identity_categories: Option<Vec<String>>,
    pub canonical_fingerprint: Option<String>,
    pub metadata_json: Option<serde_json::Value>,
}

impl MediaItem {
    /// Construct a new item with sensible empty defaults, timestamped now.
    pub fn new(media_type: MediaType, title: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: ItemId::new(),
            media_type,
            title: title.into(),
            description: None,
            language: None,
            rating_classification: RatingClassification::Unrated,
            published_at: None,
            discovered_at: now,
            updated_at: now,
            creator_ids: Vec::new(),
            series_id: None,
            source_ref_ids: Vec::new(),
            variant_ids: Vec::new(),
            tag_ids: Vec::new(),
            category_ids: Vec::new(),
            act_tag_ids: Vec::new(),
            genre_tag_ids: Vec::new(),
            production_tag_ids: Vec::new(),
            safety_status: SafetyStatus::Unreviewed,
            visibility_state: VisibilityState::Visible,
            blur_policy_id: None,
            visual_orientation: None,
            sexual_orientation_categories: None,
            participant_composition: None,
            gender_identity_categories: None,
            canonical_fingerprint: None,
            metadata_json: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let item = MediaItem::new(MediaType::Video, "Example title");
        let json = serde_json::to_string(&item).expect("serialize");
        let back: MediaItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(item.id, back.id);
        assert_eq!(item.title, back.title);
        assert_eq!(item.media_type, back.media_type);
    }
}
