use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{ItemId, SourceRefId, VariantId};

/// An actual playable, viewable, readable, cached, or downloadable
/// representation of a [`crate::media_item::MediaItem`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariant {
    pub id: VariantId,
    pub item_id: ItemId,
    pub source_ref_id: Option<SourceRefId>,
    pub local_path: Option<String>,
    pub remote_url: Option<String>,
    pub mime_type: String,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub bitrate: Option<u64>,
    pub file_size: Option<u64>,
    pub quality_label: Option<String>,
    pub language: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// Per ADR-007: a playable stream is not downloadable unless the
    /// source explicitly marks it so.
    pub download_permitted: bool,
    pub cache_permitted: bool,
    pub checksum: Option<String>,
}
