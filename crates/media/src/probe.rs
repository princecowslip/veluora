//! `ffprobe`-based media probing.
//!
//! Spawns `ffprobe` as a subprocess with a direct argument array — never
//! through a shell — per `docs/16-media-handling.md`'s external-app rule.
//! Every failure mode (binary missing, non-zero exit, unparsable output)
//! collapses to `None`: probing is always best-effort, matching the
//! scanner's existing treatment of image dimensions.

use std::path::Path;
use std::process::Command;

/// Duration/dimension/bitrate facts pulled from `ffprobe`'s JSON output.
/// Any field ffprobe didn't report (or that didn't parse) is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MediaProbe {
    pub duration_ms: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bitrate: Option<u64>,
}

/// Returns `true` if `ffprobe` is reachable on `PATH`.
pub fn ffprobe_available() -> bool {
    binary_available("ffprobe")
}

/// Returns `true` if `ffmpeg` is reachable on `PATH`.
pub fn ffmpeg_available() -> bool {
    binary_available("ffmpeg")
}

fn binary_available(program: &str) -> bool {
    Command::new(program)
        .arg("-version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Probes `path` for duration/dimensions/bitrate. Returns `None` on any
/// failure — missing binary, decode error, or unexpected output shape.
pub fn probe(path: &Path) -> Option<MediaProbe> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    Some(parse_probe_json(&json))
}

fn parse_probe_json(json: &serde_json::Value) -> MediaProbe {
    let duration_ms = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64);

    let bitrate = json
        .get("format")
        .and_then(|f| f.get("bit_rate"))
        .and_then(|b| b.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    let video_stream = json
        .get("streams")
        .and_then(|s| s.as_array())
        .and_then(|streams| {
            streams
                .iter()
                .find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("video"))
        });

    let width = video_stream
        .and_then(|s| s.get("width"))
        .and_then(|w| w.as_u64())
        .map(|w| w as u32);
    let height = video_stream
        .and_then(|s| s.get("height"))
        .and_then(|h| h.as_u64())
        .map(|h| h as u32);

    MediaProbe {
        duration_ms,
        width,
        height,
        bitrate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_and_bitrate_from_format() {
        let json = serde_json::json!({
            "format": { "duration": "12.500000", "bit_rate": "128000" },
            "streams": []
        });
        let probe = parse_probe_json(&json);
        assert_eq!(probe.duration_ms, Some(12_500));
        assert_eq!(probe.bitrate, Some(128_000));
        assert_eq!(probe.width, None);
        assert_eq!(probe.height, None);
    }

    #[test]
    fn parses_dimensions_from_the_first_video_stream() {
        let json = serde_json::json!({
            "format": {},
            "streams": [
                { "codec_type": "audio" },
                { "codec_type": "video", "width": 1920, "height": 1080 },
            ]
        });
        let probe = parse_probe_json(&json);
        assert_eq!(probe.width, Some(1920));
        assert_eq!(probe.height, Some(1080));
    }

    #[test]
    fn missing_fields_are_none_not_an_error() {
        let probe = parse_probe_json(&serde_json::json!({}));
        assert_eq!(probe, MediaProbe::default());
    }

    #[test]
    fn probing_a_nonexistent_file_returns_none_when_ffprobe_is_available() {
        if !ffprobe_available() {
            eprintln!("skipping: ffprobe not found on PATH");
            return;
        }
        assert!(probe(Path::new("/nonexistent/does-not-exist.mp4")).is_none());
    }
}
