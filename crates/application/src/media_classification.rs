//! Extension-based classification for the scanner. No content sniffing —
//! `docs/13-data-model.md`'s deeper media probing (dimensions, duration,
//! codecs beyond images) needs FFmpeg, which isn't a dependency yet and
//! is deferred to Milestone C.

use std::path::Path;

use domain::MediaType;

/// Classifies a file by extension. Returns `None` for unrecognized
/// extensions — the scanner records these as skipped/unsupported rather
/// than failing the whole scan.
pub fn classify(path: &Path) -> Option<(MediaType, &'static str)> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "mp4" => (MediaType::Video, "mp4"),
        "mkv" => (MediaType::Video, "mkv"),
        "webm" => (MediaType::Video, "webm"),
        "mov" => (MediaType::Video, "mov"),
        "avi" => (MediaType::Video, "avi"),
        "m4v" => (MediaType::Video, "m4v"),

        "jpg" | "jpeg" => (MediaType::Image, "jpg"),
        "png" => (MediaType::Image, "png"),
        "webp" => (MediaType::Image, "webp"),
        "gif" => (MediaType::Image, "gif"),
        "bmp" => (MediaType::Image, "bmp"),
        "avif" => (MediaType::Image, "avif"),

        "mp3" => (MediaType::Audio, "mp3"),
        "flac" => (MediaType::Audio, "flac"),
        "wav" => (MediaType::Audio, "wav"),
        "ogg" => (MediaType::Audio, "ogg"),
        "m4a" => (MediaType::Audio, "m4a"),
        "opus" => (MediaType::Audio, "opus"),

        "epub" => (MediaType::Story, "epub"),
        "txt" => (MediaType::Story, "txt"),
        "md" => (MediaType::Story, "md"),

        // Manga vs. Comic can't be distinguished by extension alone;
        // classify uniformly as Comic. Reclassification needs the
        // metadata-override editor, which is deferred past this
        // milestone. Archive contents (page lists) are never opened —
        // the file becomes one opaque variant.
        "cbz" => (MediaType::Comic, "cbz"),
        "cbr" => (MediaType::Comic, "cbr"),
        "cb7" => (MediaType::Comic, "cb7"),

        _ => return None,
    })
}

/// Filenames the scanner ignores outright (never even classified),
/// regardless of extension.
pub fn is_ignored_filename(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(".DS_Store") | Some("Thumbs.db") | Some("desktop.ini")
    )
}

/// The lowercase string stored in `media_items.media_type` — matches
/// `MediaType`'s `#[serde(rename_all = "snake_case")]` representation,
/// written out manually so SQL string literals elsewhere in this crate
/// (e.g. test fixtures, `search.rs`'s field mapping) have one obvious
/// source of truth to match against.
pub fn media_type_to_str(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Video => "video",
        MediaType::Image => "image",
        MediaType::Gallery => "gallery",
        MediaType::Audio => "audio",
        MediaType::Story => "story",
        MediaType::Manga => "manga",
        MediaType::Comic => "comic",
        MediaType::Other => "other",
    }
}

pub fn media_type_from_str(s: &str) -> Option<MediaType> {
    Some(match s {
        "video" => MediaType::Video,
        "image" => MediaType::Image,
        "gallery" => MediaType::Gallery,
        "audio" => MediaType::Audio,
        "story" => MediaType::Story,
        "manga" => MediaType::Manga,
        "comic" => MediaType::Comic,
        "other" => MediaType::Other,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_known_extensions_case_insensitively() {
        assert_eq!(
            classify(&PathBuf::from("a.MP4")),
            Some((MediaType::Video, "mp4"))
        );
        assert_eq!(
            classify(&PathBuf::from("a.jpeg")),
            Some((MediaType::Image, "jpg"))
        );
        assert_eq!(
            classify(&PathBuf::from("a.cbz")),
            Some((MediaType::Comic, "cbz"))
        );
    }

    #[test]
    fn unknown_extension_is_none() {
        assert_eq!(classify(&PathBuf::from("a.xyz")), None);
        assert_eq!(classify(&PathBuf::from("no_extension")), None);
    }

    #[test]
    fn ignores_known_os_artifact_filenames() {
        assert!(is_ignored_filename(&PathBuf::from("/x/.DS_Store")));
        assert!(!is_ignored_filename(&PathBuf::from("/x/video.mp4")));
    }

    #[test]
    fn media_type_string_round_trips() {
        for mt in [
            MediaType::Video,
            MediaType::Image,
            MediaType::Gallery,
            MediaType::Audio,
            MediaType::Story,
            MediaType::Manga,
            MediaType::Comic,
            MediaType::Other,
        ] {
            assert_eq!(media_type_from_str(media_type_to_str(mt)), Some(mt));
        }
    }
}
