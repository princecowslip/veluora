use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{DownloadId, ItemId, SourceId, VariantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Queued,
    Active,
    Paused,
    Completed,
    Failed,
    Canceled,
    /// Removed by quota-driven cleanup, not by the user — distinct
    /// from `Canceled` so the queue history can tell them apart.
    Evicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumState {
    Pending,
    Verified,
    Mismatch,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: DownloadId,
    pub item_id: ItemId,
    pub variant_id: VariantId,
    pub state: DownloadState,
    pub destination: String,
    pub bytes_total: Option<u64>,
    pub bytes_received: u64,
    pub checksum_state: ChecksumState,
    pub retry_count: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub failure_code: Option<String>,
    /// Denormalized from the variant's source at queue time — avoids a
    /// join on every list/poll call, and survives even if the source
    /// is later removed (`ON DELETE SET NULL`).
    pub source_id: Option<SourceId>,
    /// Exempts this specific download from `enforce_download_quota`
    /// even without pinning the whole item.
    pub pinned: bool,
    /// The in-flight `.part` file's path; `None` once finalized,
    /// canceled, or removed.
    pub temp_path: Option<String>,
    pub expected_checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    /// Resume validators from the last response, used as `If-Range` on
    /// the next resume attempt.
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
}
