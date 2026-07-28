//! Video frame extraction via `ffmpeg`, for thumbnailing. Spawns
//! `ffmpeg` with a direct argument array — never a shell — per
//! `docs/16-media-handling.md`'s external-app rule.

use std::path::Path;
use std::process::Command;

/// Extracts a single PNG-encoded frame a few seconds into `path` via
/// `ffmpeg`. Returns `None` on any failure (missing binary, a video too
/// short to seek into, corrupt container, non-zero exit) — best-effort,
/// same treatment as [`crate::probe::probe`].
pub fn extract_frame_png(path: &Path) -> Option<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args(["-v", "quiet", "-ss", "00:00:03"])
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(output.stdout)
}
