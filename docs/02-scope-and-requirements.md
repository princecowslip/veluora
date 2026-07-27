# Scope and Requirements

## Release tiers

### Tier 1: Local library

Required for the first usable release:

- Scan user-selected folders.
- Detect supported media and archives.
- Generate normalized metadata and thumbnails.
- Play video and audio.
- View images and animated images.
- Read plain text, sanitized HTML, and EPUB stories.
- Read image folders and supported comic archives.
- Save favorites, collections, notes, ratings, and private tags.
- Track progress and viewing history.
- Search locally.
- Provide a CLI.
- Provide privacy, deletion, and lock controls.

### Tier 2: Desktop experience

- Desktop GUI
- Grid and list browsing
- Reader and player shells
- Saved searches
- Download and cache management
- Duplicate grouping
- Import and export
- Source setup wizard
- Diagnostics

### Tier 3: External sources

- Connector SDK
- Official API connector
- Feed connector
- Generic booru-compatible API connector
- Per-source authentication
- Rate limiting
- Capability discovery
- Health checks
- Source-specific search translation

### Tier 4: Advanced clients and plugins

- TUI
- Sandboxed third-party plugins
- Signed plugin registry
- Remote-control API
- Optional local network access
- Optional browser extension

## Functional requirements

### Library

- Index media without moving original files.
- Support optional managed-library storage.
- Detect additions, changes, moves, and removals.
- Preserve user state when a file is moved and can be matched by hash.
- Store source attribution for imported content.
- Allow multiple files or source variants to represent one logical item.

### Browsing

- Browse by media type, source, creator, series, collection, tag, date, rating, viewed state, and download state.
- Offer grid, compact list, detailed list, and sequential reader modes.
- Collapse duplicates and variants.
- Hide blocked items before rendering thumbnails.

### Search

- Parse text and field filters.
- Support inclusion, exclusion, ranges, and saved searches.
- Run local and unified source searches.
- Expose search diagnostics when a query cannot be translated to a source.
- Avoid sending private local tags or notes to external sources.

### Playback and reading

- Persist position at safe intervals.
- Mark completion according to per-format rules.
- Allow external player and reader integration.
- Preserve volume, speed, zoom, direction, and layout preferences by media type.
- Offer keyboard-first navigation.

### Organization

- Favorites
- Collections
- Smart collections
- User ratings
- Notes
- Private tags
- Viewed and unviewed state
- Queue and watch-later state
- Duplicate review

### Source management

- Enable or disable individual sources.
- Show connector capabilities and permissions.
- Test authentication.
- Configure rate limits within safe connector limits.
- Show recent failures and degraded status.
- Remove source credentials independently of source metadata.

### Downloads and cache

- Download only when a source declares permission.
- Distinguish cache from permanent download.
- Resume where supported.
- Verify size and checksum where available.
- Enforce quotas.
- Avoid duplicate downloads.
- Preserve source metadata and canonical URL.

## Non-functional requirements

### Performance targets

Initial targets for a typical desktop computer:

- Application shell visible within 2 seconds after warm start.
- Local search results begin rendering within 300 ms for a 100,000-item library.
- Browsing remains responsive while indexing.
- Thumbnail generation is background-prioritized.
- Reader page navigation should appear immediate for preloaded pages.
- A failed source request must not block other source results.

### Reliability

- All state-changing operations are transactional.
- Interrupted indexing can resume.
- Interrupted downloads retain validated partial data where safe.
- Database migrations create a recoverable backup.
- Corrupt cache entries can be regenerated.
- Connector crashes are isolated and reported.

### Portability

- Configuration and database paths are discoverable.
- Export includes normalized metadata and user state.
- Imports should be versioned.
- The local library remains usable without internet access.

### Security

- No secret values in logs.
- No plugin receives ambient filesystem or network access.
- HTML is sanitized and script execution is disabled.
- Archives are checked for traversal and decompression abuse.
- Local API uses authentication even on localhost where practical.
- Remote access is disabled by default.

## Assumptions

- Users are adults under the laws applicable to them.
- Users are responsible for having lawful access to configured sources.
- Connectors cannot establish the legality or consent status of every external item.
- Officially distributed connectors are reviewed and may be disabled if the source becomes unsafe or incompatible.
