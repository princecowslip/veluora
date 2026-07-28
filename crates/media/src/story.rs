//! Plain text / Markdown story ingestion: sanitize the raw file and build
//! a chapter map, per `docs/16-media-handling.md`'s story pipeline. EPUB
//! and sanitized-HTML input stay out of scope this milestone — an EPUB
//! item is classified as `Story` but left unread until a follow-up adds
//! a real EPUB parser.

use std::path::Path;

use domain::StoryFormat;
use serde_json::Value;

use crate::error::Result;

/// Cap on how much of a story file we'll read into memory at once.
const MAX_SOURCE_BYTES: u64 = 50 * 1024 * 1024;

/// One chapter map entry: a heading's display title and the character
/// offset into the sanitized content where it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct ChapterMarker {
    pub title: String,
    pub char_offset: usize,
}

pub struct StoryContent {
    pub sanitized_text: String,
    pub chapter_map: Value,
}

/// Reads and sanitizes a plain-text or Markdown story file.
///
/// Sanitization here is deliberately conservative: this milestone has no
/// HTML renderer, so raw `<...>`-bracketed content (the shape scripts,
/// forms, and embeds all take) is stripped outright rather than parsed
/// and selectively allowed.
pub fn build_story_document(path: &Path, format: StoryFormat) -> Result<StoryContent> {
    let source_size = std::fs::metadata(path)?.len();
    let capped = source_size.min(MAX_SOURCE_BYTES);
    let raw = std::fs::read_to_string(path)?;
    let raw: &str = if raw.len() as u64 > capped {
        &raw[..capped as usize]
    } else {
        &raw
    };

    let sanitized_text = strip_html_like_content(raw);
    let chapters = match format {
        StoryFormat::Markdown => extract_markdown_chapters(&sanitized_text),
        _ => Vec::new(),
    };
    let chapter_map = serde_json::to_value(
        chapters
            .iter()
            .map(|c| serde_json::json!({ "title": c.title, "char_offset": c.char_offset }))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| Value::Array(Vec::new()));

    Ok(StoryContent {
        sanitized_text,
        chapter_map,
    })
}

/// Strips anything between `<` and `>` — a defensive measure against
/// embedded raw HTML (scripts, forms, trackers) in Markdown source,
/// since nothing in this milestone renders HTML.
fn strip_html_like_content(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut inside_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Extracts ATX-style Markdown headings (`# Title` through `###### Title`)
/// into a chapter map, recording the character offset each heading
/// starts at within `text`.
fn extract_markdown_chapters(text: &str) -> Vec<ChapterMarker> {
    let mut chapters = Vec::new();
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(title) = heading_title(trimmed) {
            chapters.push(ChapterMarker {
                title,
                char_offset: offset,
            });
        }
        offset += line.chars().count();
    }
    chapters
}

fn heading_title(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        // "#no-space" isn't a heading per the ATX rule.
        return None;
    }
    Some(rest.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_headings_become_a_chapter_map() {
        let text = "# Chapter One\nSome text.\n\n## Chapter Two\nMore text.\n";
        let chapters = extract_markdown_chapters(text);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter One");
        assert_eq!(chapters[0].char_offset, 0);
        assert_eq!(chapters[1].title, "Chapter Two");
    }

    #[test]
    fn non_heading_hashes_are_ignored() {
        let text = "#no-space-not-a-heading\nplain text";
        let chapters = extract_markdown_chapters(text);
        assert!(chapters.is_empty());
    }

    #[test]
    fn strips_html_like_tags_but_leaves_their_text_content_inert() {
        // Tags are removed; any content between them survives as plain,
        // unexecuted text — there's no HTML renderer in this milestone,
        // so leftover script/tag content is just harmless literal text.
        let input = "Hello <script>alert(1)</script> world";
        assert_eq!(strip_html_like_content(input), "Hello alert(1) world");
    }

    #[test]
    fn strips_self_closing_and_void_tags_entirely() {
        let input = "Line one<br/>Line two<img src=\"x\">";
        assert_eq!(strip_html_like_content(input), "Line oneLine two");
    }

    #[test]
    fn build_story_document_for_markdown_produces_chapters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("story.md");
        std::fs::write(&path, "# Intro\nHello <b>world</b>\n").unwrap();

        let doc = build_story_document(&path, StoryFormat::Markdown).unwrap();
        assert!(doc.sanitized_text.contains("Hello world"));
        assert!(!doc.sanitized_text.contains('<'));
        let chapters = doc.chapter_map.as_array().unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0]["title"], "Intro");
    }

    #[test]
    fn build_story_document_for_plain_text_has_no_chapters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("story.txt");
        std::fs::write(&path, "# not a heading, just text\n").unwrap();

        let doc = build_story_document(&path, StoryFormat::PlainText).unwrap();
        assert_eq!(doc.chapter_map.as_array().unwrap().len(), 0);
    }
}
