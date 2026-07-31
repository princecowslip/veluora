# Implementation Plan

## Objective

Turn the Veloura planning package into a buildable, testable, local-first application while preserving the privacy, safety, source-capability, and interface-consistency requirements defined in this repository.

## Delivery strategy

Build vertical slices rather than completing isolated subsystems.

Each slice should include:

- Domain model
- Application service
- Database migration
- Local API operation
- CLI command
- GUI workflow
- TUI workflow where scheduled
- Tests
- Documentation
- Privacy and security review

## Workstream 1: Repository and build foundation

Deliver:

- Monorepo or coordinated multi-package workspace
- Core service package
- CLI package
- GUI package
- notcurses TUI package
- Connector SDK
- Database migrations
- Fixture library
- CI pipelines
- Packaging directories
- Dependency lockfiles
- Software bill of materials generation

Acceptance:

- Clean checkout builds on Tier 1 platforms.
- Unit tests and formatting run in CI.
- Debug and release profiles exist.
- Build outputs do not embed credentials or local paths.

## Workstream 2: Domain core

Implement:

- Media item and variant
- Source reference
- Creator, performer, series, gallery, and chapter
- User state
- Collection and smart collection
- Progress
- Tag namespaces
- Orientation and category taxonomy
- Block rules
- Download state
- Source capability model

Acceptance:

- Entities serialize to versioned API structures.
- User-owned metadata remains distinguishable from source metadata.
- Block rules can be evaluated without presentation code.
- Real-person identity and orientation are never inferred by the domain layer.

## Workstream 3: Local storage

Implement:

- SQLite schema
- FTS5 indexes
- Migration runner
- Transaction boundaries
- Backup before migration
- Cache directory structure
- Thumbnail index
- Credential-store adapter
- Import/export format

Acceptance:

- Migrations are reversible or have a tested restore path.
- Search scales to the target library size.
- Secrets do not enter SQLite.
- Deletion tests verify database, indexes, thumbnails, cache, and temporary files.

## Workstream 4: Local library

Implement:

- Folder roots
- Scanner
- Watch folders
- MIME validation
- Media probing
- Archive inspection
- Hashing and fingerprinting
- File move detection
- Metadata override editor
- Duplicate grouping

Acceptance:

- Interrupted scans resume safely.
- Moved files retain user state when matched.
- Unsupported files are reported without failing the scan.
- Archive traversal and decompression limits are enforced.

## Workstream 5: Playback and readers

Implement:

- Video and audio playback coordination
- Image viewer
- Animated image support
- Story reader
- Manga and comic reader
- External player adapters
- Resume and completion rules
- Thumbnail generation

Acceptance:

- Every supported media type opens and resumes.
- Viewer failures preserve progress.
- External process arguments avoid shell interpolation.
- Locked mode covers or destroys sensitive preview surfaces.

## Workstream 6: Search and organization

Implement:

- Query parser and AST
- Local search translation
- Filter chips
- Saved searches
- Favourites
- Ratings
- Notes
- Private tags
- Manual collections
- Smart collections
- Queue
- Batch actions

Acceptance:

- GUI, CLI, and TUI use the same query semantics.
- Private tags and notes never leave the local search scope.
- Search errors identify the invalid query segment.
- Reversible organization actions provide Undo in graphical interfaces.

## Workstream 7: Desktop GUI

Implement:

- Application shell
- Onboarding
- Home and feed
- Library
- Discover
- Item details
- Viewer shells
- Collections
- Downloads
- Sources
- Privacy Center
- Settings
- Diagnostics

Acceptance:

- A new user completes local setup without documentation.
- Public-source feeds remain opt-in.
- Keyboard-only flows pass.
- Simple mode hides technical complexity.
- Advanced mode does not weaken safety rules.

## Workstream 8: CLI

Implement:

- Stable command tree
- Text, table, JSON, and JSONL output
- Shell completion
- Exit codes
- Secure interactive authentication
- Search and filter commands
- Library and collection commands
- Privacy and diagnostic commands

Acceptance:

- JSON output includes schema version.
- Passwords and tokens are not accepted as ordinary command arguments.
- Non-interactive failures return documented exit codes.
- Plain output works without colour.

## Workstream 9: notcurses TUI

Implement:

- C++20 client
- notcurses lifecycle
- Responsive plane hierarchy
- Input map
- Search overlay
- Library reel and table
- Detail pane
- Home feed
- Downloads
- Privacy shield
- Capability-tiered previews
- Service reconnect

Acceptance:

- Terminal is restored on every tested exit path.
- Text-only mode supports all essential workflows.
- Locked mode hides titles, tags, and previews.
- TUI never accesses SQLite or credentials directly.
- Linux and macOS Tier 1 terminal matrices pass.

## Workstream 10: Connectors

Implement first:

- Local filesystem
- Feed
- OPDS
- Stash
- Hydrus
- Jellyfin
- Kavita
- Komga
- One Danbooru-family connector
- One Gelbooru-family connector
- Metadata-only Stash-box connector
- Browser handoff template

Acceptance:

- Connectors declare capabilities and domains.
- Connector failures are isolated.
- Unsupported filters are reported.
- Download appears only when explicitly permitted.
- Source attribution remains visible.
- Credentials are source-scoped.

## Workstream 11: Downloads and offline use

Implement:

- Eligibility checks
- Download queue
- Pause and resume
- Verification
- Temporary files
- Naming templates
- Cache and permanent-download distinction
- Quotas
- Offline state

Acceptance:

- Partial files never appear as complete.
- Interrupted downloads recover where the source supports ranges.
- Cache cleanup never removes pinned or permanent files unless configured.
- Deleting a download can preserve the library reference.

## Workstream 12: Plugins

Implement after connector stabilization:

- Manifest
- Signature validation
- Permissions
- Separate host
- Domain allowlists
- Credential handles
- Resource limits
- Crash disablement
- Revocation list
- Developer tooling

Acceptance:

- Plugins cannot access undeclared domains or files.
- Permission changes require fresh approval.
- Repeated crashes disable the plugin.
- Unsigned plugins require explicit developer mode.

## Workstream 13: Quality and release

Implement:

- Unit, integration, contract, end-to-end, security, privacy, accessibility, and performance tests
- Signed packages
- Upgrade and rollback tests
- Redacted support bundle
- Changelog
- Known issues
- Release notes
- Dependency inventory
- SBOM
- Security scan

Acceptance:

- All release gates in `28-release-checklist.md` and `39-release-polish-checklist.md` pass.
- No known credential leak, destructive migration, broken deletion, or unrestricted network binding remains.
- Documentation matches shipped behaviour.

## Suggested milestone sequence

### Milestone A — Foundation

Repository, CI, domain types, SQLite, local API skeleton, CLI skeleton.

### Milestone B — Local library

Scanning, local search, metadata, thumbnails, favourites, collections.

### Milestone C — Media experience

Video, audio, images, stories, manga, comics, progress.

### Milestone D — Desktop MVP

Onboarding, Home, Library, Viewer, Privacy Center, Settings.

### Milestone E — Privacy and reliability

Encryption option, deletion verification, diagnostics, backup and restore.

### Milestone F — Connectors

Personal servers, feeds, booru APIs, metadata providers, browser handoff.

### Milestone G — TUI and downloads

notcurses TUI, authorized downloads, offline mode.

### Milestone H — Plugin preview

Sandboxed third-party connector preview and registry governance.

### Milestone I — Unified Discover

A `DiscoverService` aggregating the local library with every enabled
connector source in one call, plus `local-api`/CLI/GUI/TUI surfaces —
closing the gap left by Milestone F shipping connectors and per-source
browse (`SourceService::browse`) without a unified cross-source query.

### Milestone J — Downloads and offline use

Workstream 11 made real: `FeedConnector` extended to declare
`capabilities().downloads` and surface a per-entry enclosure URL, a
`DownloadService` with eligibility checks, an atomic temp-file →
verified-rename fetch/resume engine coordinated through the shared
database rather than an in-memory flag (no background daemon exists in
this codebase), quota enforcement sharing its eviction loop with
Milestone G's cache quota, and `local-api`/CLI/GUI/TUI surfaces —
closing the gap `KNOWN_ISSUES.md` flagged since Milestone F shipped
connectors with no connector able to exercise a real download.

## Definition of done

A feature is complete only when:

- User behaviour is documented.
- Domain and application logic are tested.
- Privacy and safety implications are reviewed.
- GUI and applicable CLI/TUI surfaces exist.
- Errors and recovery are designed.
- Accessibility is tested.
- Telemetry and logging implications are reviewed.
- Migration and deletion effects are tested.
