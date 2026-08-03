# Media Handling

## General media pipeline

```text
Acquire reference
→ validate URL or path
→ inspect MIME and container
→ enforce size limits
→ derive metadata
→ select decoder or player
→ render
→ persist progress
→ release temporary access
```

## Video

### Supported behavior

- Local files
- Authorized direct streams
- Source-provided HLS or DASH
- Multiple quality variants
- Audio tracks
- Subtitles
- Playback speed
- Resume
- External player

### Playback rules

- Prefer stable local variants.
- Refresh expiring remote variants just before playback.
- Persist progress every 10–30 seconds and on pause, seek, or exit.
- Mark complete using a configurable threshold, such as 90–95%.
- Never treat the presence of a stream URL as download authorization.

### Failure states

- Expired URL
- Codec unsupported
- Network interrupted
- Authentication expired
- Source removed
- File moved
- Corrupt container

## Images and animated images

Features:

- Progressive decoding where available
- Large-image tiling
- Zoom, pan, rotate
- Fit modes
- Slideshow
- Animation pause
- Frame stepping where practical
- Color profile handling

Security:

- Validate decoded dimensions before allocation.
- Cap decompressed image size.
- Isolate risky decoders.
- Ignore embedded scripts or active content.

## Stories

Shipped (`crates/media/src/story.rs`): plain text and Markdown only.
Sanitized HTML and EPUB input remain unbuilt (`KNOWN_ISSUES.md`, Media
handling section) — the fuller list below remains the design target:

Accepted input may include:

- Plain text
- Markdown
- Sanitized HTML
- EPUB
- Source-provided chapters

Pipeline:

1. Extract text and structure.
2. Remove scripts, forms, trackers, and unsafe embeds.
3. Normalize headings and paragraphs.
4. Build chapter map.
5. Build full-text index.
6. Store safe local representation.
7. Render with reader settings.

## Audio

Features:

- Standard playback controls
- Speed
- Chapter navigation
- Queue
- Sleep timer
- Resume
- Cover art
- Waveform or seek map where available
- External player

Completion should use duration threshold or explicit user action.

## Manga and comics

Shipped (`crates/media/src/archive.rs`): CBZ (ZIP-based) comic archives
only — no CBR, CB7, or EPUB comics support yet (`KNOWN_ISSUES.md`, Media
handling section). The fuller list below remains the design target:

Input types:

- Image folders
- ZIP-based comic archives
- EPUB comics
- Source galleries
- Chapter page lists

Reader modes:

- Single page
- Double page
- Right-to-left
- Left-to-right
- Vertical strip
- Fit width
- Fit height
- Original size
- Margin crop

Archive safety:

- Reject absolute paths.
- Reject parent traversal.
- Limit entry count.
- Limit uncompressed size.
- Limit compression ratio.
- Avoid executable extraction.
- Extract to controlled temporary directories.

## Thumbnails

Thumbnail generation must:

- Use bounded resolution.
- Strip unnecessary metadata.
- Avoid preserving source credentials in URLs.
- Respect private and neutral modes.
- Be regenerable.
- Store a version key tied to thumbnail settings.

## External applications

External commands receive:

- Local path when available
- Temporary authorized URL only when safe
- Item identifier
- Optional subtitle path

Use direct argument arrays and avoid shell evaluation.
