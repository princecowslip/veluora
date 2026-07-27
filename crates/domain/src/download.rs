use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{DownloadId, ItemId, VariantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Queued,
    Active,
    Paused,
    Completed,
    Failed,
    Canceled,
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
}
