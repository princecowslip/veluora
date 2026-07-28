//! Thumbnail generation for images, video frames, and comic pages.
//!
//! Storage path is versioned (`cache/thumbnails/v{VERSION}/...`) so
//! bumping [`THUMBNAIL_SETTINGS_VERSION`] naturally invalidates every
//! existing thumbnail via path change — no separate invalidation pass
//! needed. Matches the `cache/thumbnails/` convention in
//! `docs/17-downloads-cache-storage.md`.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use domain::VariantId;
use image::{ImageReader, Limits};

use crate::context::AppContext;
use crate::error::{AppError, Result};

pub const THUMBNAIL_SETTINGS_VERSION: u32 = 1;
const MAX_DIMENSION: u32 = 512;
/// Refuse to even attempt decoding source files larger than this —
/// a decompression-bomb guard per `docs/16-media-handling.md`.
const MAX_SOURCE_BYTES: u64 = 100 * 1024 * 1024;

pub struct ThumbnailService;

impl ThumbnailService {
    /// Returns the cache path for `variant_id`'s thumbnail, generating it
    /// from `source_path` (an image file) first if it doesn't already
    /// exist.
    pub fn ensure(ctx: &AppContext, variant_id: VariantId, source_path: &Path) -> Result<PathBuf> {
        let out_path = Self::cache_path(ctx, variant_id);
        if out_path.exists() {
            return Ok(out_path);
        }
        let source_size = std::fs::metadata(source_path)?.len();
        if source_size > MAX_SOURCE_BYTES {
            return Err(AppError::InvalidPath(format!(
                "{} exceeds the {MAX_SOURCE_BYTES}-byte thumbnail source limit",
                source_path.display()
            )));
        }
        let bytes = std::fs::read(source_path)?;
        generate_from_bytes(&bytes, &out_path)?;
        Ok(out_path)
    }

    /// Generates `variant_id`'s thumbnail from a frame extracted a few
    /// seconds into the video at `source_path`, via `ffmpeg`. Best-effort
    /// — returns [`AppError::InvalidPath`] if `ffmpeg` is missing, the
    /// video is too short to seek into, or the frame can't be decoded.
    pub fn ensure_video_frame(
        ctx: &AppContext,
        variant_id: VariantId,
        source_path: &Path,
    ) -> Result<PathBuf> {
        let out_path = Self::cache_path(ctx, variant_id);
        if out_path.exists() {
            return Ok(out_path);
        }
        let bytes = media::extract_frame_png(source_path).ok_or_else(|| {
            AppError::InvalidPath(format!(
                "could not extract a video frame from {}",
                source_path.display()
            ))
        })?;
        generate_from_bytes(&bytes, &out_path)?;
        Ok(out_path)
    }

    /// Generates `variant_id`'s thumbnail from a comic archive's page 0.
    pub fn ensure_comic_page(
        ctx: &AppContext,
        variant_id: VariantId,
        archive_path: &Path,
    ) -> Result<PathBuf> {
        let out_path = Self::cache_path(ctx, variant_id);
        if out_path.exists() {
            return Ok(out_path);
        }
        let bytes = media::read_page(archive_path, 0).map_err(|e| {
            AppError::InvalidPath(format!(
                "could not read page 0 of {}: {e}",
                archive_path.display()
            ))
        })?;
        generate_from_bytes(&bytes, &out_path)?;
        Ok(out_path)
    }

    pub fn cache_path(ctx: &AppContext, variant_id: VariantId) -> PathBuf {
        // Sharded by the first two hex characters of the variant id so a
        // single directory doesn't accumulate 100k+ entries at scale.
        let id = variant_id.to_string();
        let shard = &id[..2.min(id.len())];
        ctx.data_dir
            .join("cache")
            .join("thumbnails")
            .join(format!("v{THUMBNAIL_SETTINGS_VERSION}"))
            .join(shard)
            .join(format!("{id}.jpg"))
    }
}

/// Decodes, bounds-checks, resizes, and re-encodes a thumbnail from an
/// in-memory image buffer — shared by the image, video-frame, and
/// comic-page generation paths above.
fn generate_from_bytes(bytes: &[u8], out_path: &Path) -> Result<()> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::InvalidPath(format!("could not guess image format: {e}")))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| AppError::InvalidPath(format!("could not decode thumbnail source: {e}")))?;

    // Never upscale: only resize when the source actually exceeds the cap.
    let thumbnail = if image.width() > MAX_DIMENSION || image.height() > MAX_DIMENSION {
        image.thumbnail(MAX_DIMENSION, MAX_DIMENSION)
    } else {
        image
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Re-encoding from the decoded pixel buffer is what strips EXIF/ICC —
    // no source metadata bytes ever enter the output file.
    thumbnail
        .save_with_format(out_path, image::ImageFormat::Jpeg)
        .map_err(|e| {
            AppError::InvalidPath(format!(
                "could not write thumbnail {}: {e}",
                out_path.display()
            ))
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thumbnails need a real filesystem `data_dir` (unlike most other
    /// services, which are fine with `AppContext::open_in_memory()`) —
    /// backing it with an in-memory DB's placeholder `:memory:` path
    /// would create a literal `./:memory:/` directory on disk.
    fn test_ctx() -> (AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open_at(dir.path()).unwrap();
        (ctx, dir)
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([200, 100, 50]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(path, image::ImageFormat::Png)
            .unwrap();
    }

    #[test]
    fn generates_a_jpeg_thumbnail_bounded_to_max_dimension() {
        let (ctx, _ctx_dir) = test_ctx();
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("big.png");
        write_test_png(&src_path, 1000, 400);

        let variant_id = VariantId::new();
        let out_path = ThumbnailService::ensure(&ctx, variant_id, &src_path).unwrap();
        assert!(out_path.exists());

        let (w, h) = image::image_dimensions(&out_path).unwrap();
        assert!(w <= MAX_DIMENSION && h <= MAX_DIMENSION);
        assert_eq!(
            image::ImageReader::open(&out_path)
                .unwrap()
                .with_guessed_format()
                .unwrap()
                .format(),
            Some(image::ImageFormat::Jpeg)
        );
    }

    #[test]
    fn never_upscales_a_small_source_image() {
        let (ctx, _ctx_dir) = test_ctx();
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("small.png");
        write_test_png(&src_path, 40, 30);

        let variant_id = VariantId::new();
        let out_path = ThumbnailService::ensure(&ctx, variant_id, &src_path).unwrap();
        let (w, h) = image::image_dimensions(&out_path).unwrap();
        assert_eq!((w, h), (40, 30));
    }

    #[test]
    fn ensure_is_idempotent_and_reuses_the_cached_file() {
        let (ctx, _ctx_dir) = test_ctx();
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("pic.png");
        write_test_png(&src_path, 100, 100);

        let variant_id = VariantId::new();
        let first = ThumbnailService::ensure(&ctx, variant_id, &src_path).unwrap();
        let modified_before = std::fs::metadata(&first).unwrap().modified().unwrap();

        // Second call with a nonexistent source path still succeeds
        // because the cached thumbnail already exists.
        let second = ThumbnailService::ensure(&ctx, variant_id, Path::new("/nonexistent")).unwrap();
        assert_eq!(first, second);
        let modified_after = std::fs::metadata(&second).unwrap().modified().unwrap();
        assert_eq!(modified_before, modified_after);
    }

    #[test]
    fn rejects_oversized_source_files() {
        let (ctx, _ctx_dir) = test_ctx();
        let src_dir = tempfile::tempdir().unwrap();
        let src_path = src_dir.path().join("huge.png");
        // Cheaply exceed the limit without actually writing 100MB: sparse
        // file via set_len.
        let file = std::fs::File::create(&src_path).unwrap();
        file.set_len(MAX_SOURCE_BYTES + 1).unwrap();

        let variant_id = VariantId::new();
        let err = ThumbnailService::ensure(&ctx, variant_id, &src_path).unwrap_err();
        assert!(matches!(err, AppError::InvalidPath(_)));
    }

    #[test]
    fn ensure_comic_page_thumbnails_the_first_page_of_a_cbz() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;

        let (ctx, _ctx_dir) = test_ctx();
        let src_dir = tempfile::tempdir().unwrap();
        let cbz_path = src_dir.path().join("book.cbz");

        let mut page_png = Vec::new();
        write_test_png_to(&mut page_png, 300, 200);

        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("000.png", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&page_png).unwrap();
        writer.finish().unwrap();

        let variant_id = VariantId::new();
        let out_path = ThumbnailService::ensure_comic_page(&ctx, variant_id, &cbz_path).unwrap();
        assert!(out_path.exists());
        let (w, h) = image::image_dimensions(&out_path).unwrap();
        assert_eq!((w, h), (300, 200));
    }

    fn write_test_png_to(buf: &mut Vec<u8>, width: u32, height: u32) {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([10, 20, 30]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(buf), image::ImageFormat::Png)
            .unwrap();
    }
}
