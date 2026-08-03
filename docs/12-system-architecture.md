# System Architecture

## Overview

The system uses a shared core with interface adapters.

```text
GUI ─┐
TUI ─┼── Application Services ── Domain Core ── Repositories
CLI ─┘             │                 │               │
                   │                 │               ├─ SQLite
                   │                 │               ├─ Filesystem
                   │                 │               └─ Credential Store
                   │                 │
                   │                 ├─ Search
                   │                 ├─ Library
                   │                 ├─ Playback
                   │                 ├─ Downloads
                   │                 ├─ Safety
                   │                 └─ User State
                   │
                   └─ Connector Runtime ── Official and Plugin Connectors
```

## Recommended process model

### Main application process

Responsible for:

- UI
- command routing
- application services
- database access
- user-state changes
- local API authorization

### Worker process

Responsible for:

- thumbnail generation
- metadata probing
- archive inspection
- media fingerprinting
- download jobs
- connector execution where isolation is required

### Media engine process

Use an established engine for playback. It may run embedded or as a controlled external process.

### Plugin host process

Third-party connectors run in a separate host with:

- restricted environment
- scoped credentials
- domain allowlist
- time and memory limits
- structured request and response messages

## Architectural layers

### Presentation

- GUI screens and components
- TUI panes and key maps
- CLI command handlers

Presentation code must not call connector or database implementations directly.

### Application services

Shipped, in `crates/application/src/`:

- SearchService
- LibraryService
- LibraryRootService
- ScanService
- ItemService
- CollectionService
- ComicService
- StoryService
- ThumbnailService
- UserStateService
- PlaybackService
- DownloadService
- SourceService
- DiscoverService
- SettingsService
- PrivacyService
- DiagnosticsService

There is no dedicated `SafetyService` or `PluginService` — safety/blocking
logic is consulted ad hoc inside `DownloadService`'s eligibility check, and
plugin governance (manifest validation, permissions, the sandbox) lives
entirely in the separate `crates/plugin-host` crate rather than as an
`application` service.

### Domain core

Contains:

- entities
- value objects
- policies
- capability checks
- query model
- completion rules
- blocking rules
- duplicate grouping
- download eligibility

### Infrastructure

- SQLite repositories
- filesystem scanner
- credential manager adapter
- media probe adapter
- thumbnail generator
- HTTP client
- connector host
- event bus
- logging

## Event model

Useful domain events:

```text
LibraryRootAdded
IndexingStarted
ItemDiscovered
ItemMetadataUpdated
ThumbnailReady
ProgressUpdated
ItemCompleted
CollectionChanged
SourceHealthChanged
DownloadQueued
DownloadCompleted
BlockRuleChanged
PrivacyDataCleared
PluginDisabled
```

Events should not include credentials or unnecessary explicit text.

## Concurrency

- Use bounded job queues.
- Prioritize foreground playback and reading over indexing.
- Limit per-source concurrent requests.
- Cancel stale search requests.
- Write user state transactionally.
- Coalesce repeated progress updates.
- Use backpressure for thumbnail and fingerprint queues.

## Offline behavior

When offline:

- Local library remains fully available.
- Cached media remains available according to policy.
- Source searches are skipped with a clear status.
- Pending source actions remain queued only when safe and expected.
- Expiring URLs are not treated as permanent media records.

## Deployment modes

### Embedded desktop

All components are started by the GUI.

### Local service

A background local daemon serves GUI, TUI, and CLI clients.

### Headless library server

A future, separate mode for advanced users. It must not be enabled by default and requires authenticated, encrypted access.

## Technology selection criteria

Choose technologies based on:

- Memory safety
- Cross-platform support
- Mature SQLite integration
- Strong terminal framework
- Reliable media engine integration
- Sandboxing options
- Packaging and automatic update support
- Long-term maintainability

Avoid making the core depend on a single GUI toolkit.

## notcurses TUI boundary

The terminal client is implemented in C++20 with notcurses 3.x.

It communicates with the shared application service through authenticated local IPC or loopback HTTP. The TUI does not link the core domain implementation, open the application database, run source connectors, or retrieve credentials directly.

This separation:

- Contains terminal-specific native dependencies.
- Avoids notcurses FFI in the core.
- Lets the TUI restart independently.
- Makes native Windows and WSL packaging separable.
- Preserves identical privacy and safety decisions across interfaces.
