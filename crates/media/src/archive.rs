//! CBZ (zip-based comic archive) page listing and extraction.
//!
//! Applies the archive-safety rules from `docs/16-media-handling.md`:
//! reject absolute/traversal entry paths, cap entry count, cap total
//! uncompressed size, and cap per-entry compression ratio (zip-bomb
//! guard). Rather than "extract to controlled temporary directories",
//! [`read_page`] reads one bounded entry straight into memory — it meets
//! the same safety goal (bounded, no traversal, nothing touches disk
//! outside the archive) without temp-file lifecycle management.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::error::{MediaError, Result};

const MAX_ENTRIES: usize = 10_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 100;
const MAX_PAGE_BYTES: u64 = 50 * 1024 * 1024;

/// One image entry inside a comic archive, in reading order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePage {
    pub index: u32,
    pub entry_name: String,
    pub size: u64,
}

/// Lists the image entries of a CBZ archive, sorted lexicographically by
/// entry name (the standard comic-archive reading order), with
/// `index` assigned by that sorted position.
pub fn list_pages(path: &Path) -> Result<Vec<ArchivePage>> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;
    validate_archive(&mut archive)?;

    let mut pages = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if !is_image_entry(&name) {
            continue;
        }
        pages.push(ArchivePage {
            index: 0,
            entry_name: name,
            size: entry.size(),
        });
    }
    pages.sort_by(|a, b| a.entry_name.cmp(&b.entry_name));
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i as u32;
    }
    Ok(pages)
}

/// Reads one page's raw bytes into memory, bounded by [`MAX_PAGE_BYTES`].
pub fn read_page(path: &Path, index: u32) -> Result<Vec<u8>> {
    let pages = list_pages(path)?;
    let page = pages
        .into_iter()
        .find(|p| p.index == index)
        .ok_or(MediaError::PageNotFound(index))?;
    if page.size > MAX_PAGE_BYTES {
        return Err(MediaError::ArchiveTooLarge(format!(
            "page {index} is {} bytes, exceeding the {MAX_PAGE_BYTES}-byte page limit",
            page.size
        )));
    }

    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;
    let mut entry = archive.by_name(&page.entry_name)?;
    let mut bytes = Vec::with_capacity(page.size as usize);
    // Read one byte past the declared size to catch a header that lied.
    entry
        .by_ref()
        .take(MAX_PAGE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PAGE_BYTES {
        return Err(MediaError::ArchiveTooLarge(format!(
            "page {index} exceeded the {MAX_PAGE_BYTES}-byte page limit while reading"
        )));
    }
    Ok(bytes)
}

fn validate_archive<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<()> {
    if archive.len() > MAX_ENTRIES {
        return Err(MediaError::ArchiveTooLarge(format!(
            "{} entries exceeds the {MAX_ENTRIES}-entry limit",
            archive.len()
        )));
    }

    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        check_entry_name_is_safe(&name)?;

        let uncompressed = entry.size();
        let compressed = entry.compressed_size().max(1);
        total_uncompressed = total_uncompressed.saturating_add(uncompressed);
        if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(MediaError::ArchiveTooLarge(format!(
                "total uncompressed size exceeds the {MAX_TOTAL_UNCOMPRESSED_BYTES}-byte limit"
            )));
        }
        if uncompressed / compressed > MAX_COMPRESSION_RATIO {
            return Err(MediaError::ArchiveTooLarge(format!(
                "entry '{name}' has a compression ratio exceeding {MAX_COMPRESSION_RATIO}x"
            )));
        }
    }
    Ok(())
}

fn check_entry_name_is_safe(name: &str) -> Result<()> {
    let path = Path::new(name);
    if path.is_absolute() {
        return Err(MediaError::PathTraversal(name.to_string()));
    }
    for component in path.components() {
        match component {
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(MediaError::PathTraversal(name.to_string()));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

fn is_image_entry(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn write_cbz(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn lists_pages_in_sorted_order_and_skips_non_images() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.cbz");
        write_cbz(
            &path,
            &[
                ("002.jpg", b"b"),
                ("000.jpg", b"a"),
                ("001.jpg", b"c"),
                ("ComicInfo.xml", b"<xml/>"),
            ],
        );

        let pages = list_pages(&path).unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].entry_name, "000.jpg");
        assert_eq!(pages[1].entry_name, "001.jpg");
        assert_eq!(pages[2].entry_name, "002.jpg");
        assert_eq!(pages[0].index, 0);
        assert_eq!(pages[2].index, 2);
    }

    #[test]
    fn read_page_returns_the_right_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.cbz");
        write_cbz(&path, &[("000.jpg", b"hello page"), ("001.jpg", b"second")]);

        let bytes = read_page(&path, 0).unwrap();
        assert_eq!(bytes, b"hello page");
        let bytes = read_page(&path, 1).unwrap();
        assert_eq!(bytes, b"second");
    }

    #[test]
    fn read_page_out_of_range_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.cbz");
        write_cbz(&path, &[("000.jpg", b"only page")]);

        let err = read_page(&path, 5).unwrap_err();
        assert!(matches!(err, MediaError::PageNotFound(5)));
    }

    #[test]
    fn rejects_entries_with_parent_directory_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("evil.cbz");
        write_cbz(&path, &[("../../etc/passwd", b"nope")]);

        let err = list_pages(&path).unwrap_err();
        assert!(matches!(err, MediaError::PathTraversal(_)));
    }

    #[test]
    fn rejects_archives_with_too_many_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.cbz");
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for i in 0..(MAX_ENTRIES + 1) {
            writer.start_file(format!("{i:06}.jpg"), options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();

        let err = list_pages(&path).unwrap_err();
        assert!(matches!(err, MediaError::ArchiveTooLarge(_)));
    }

    #[test]
    fn rejects_a_zip_bomb_style_compression_ratio() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bomb.cbz");
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        // Highly compressible content (all zeros) compresses far beyond
        // MAX_COMPRESSION_RATIO with a real deflate implementation.
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("000.jpg", options).unwrap();
        let payload = vec![0u8; 50 * 1024 * 1024];
        writer.write_all(&payload).unwrap();
        writer.finish().unwrap();

        let err = list_pages(&path).unwrap_err();
        assert!(matches!(err, MediaError::ArchiveTooLarge(_)));
    }
}
