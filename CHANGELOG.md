# Changelog

Format loosely follows [Keep a Changelog](https://keepachangelog.com/). Veloura hasn't cut a versioned release yet — this tracks what each milestone actually shipped. See `KNOWN_ISSUES.md` for what's explicitly out of scope so far.

## [Unreleased]

### Workstream 13 — Quality and release (hardening pass)

- `FeedConnector` now enforces a response-size cap, preventing a malicious or misconfigured feed from exhausting memory.
- Regression tests locking in existing-but-previously-untested guarantees: `local-api` never returns a permissive CORS header, CLI text output never contains ANSI escape codes, the redacted support bundle excludes notes/private tags (not just titles/paths).
- A real "upgrade" test: a database pinned at migration `0001` with real data survives being opened through the full `0002`–`0005` migration chain.
- `cargo-deny` wired in (`deny.toml`) for dependency license/advisory scanning, plus a dependency inventory.
- CI now builds the C++ TUI (`tui/`) on every push — previously untested in CI.
- This file and `KNOWN_ISSUES.md`.

## Milestone F — Connectors (PR #8)

- New `crates/connectors`: an async `Connector` trait, a `ConnectorRegistry`, and `FeedConnector` (RSS/Atom) — a real network connector, fetch-tested against a genuine local HTTP server.
- `application::source::SourceService`: CRUD on `sources`, a `LocalFilesystemConnector` wrapping the existing `SearchService`, query-translation reporting for connectors that can't search, and `import_remote_item` — the first code to write to `source_references`.
- `local-api` `/api/v1/sources` routes and `veloura source` CLI commands (list/add/remove/enable/disable/health-check/browse/import).

## Milestone H — Plugin sandbox/governance infrastructure (PR #7)

- New `crates/plugin-host`: a plugin manifest schema/validator (`docs/18-plugin-system.md`), a default-deny permission type model, a local file-backed plugin registry with a Stable/Beta/Degraded/Disabled/Removed status lifecycle, and a real WASM sandbox (`wasmtime`) — default-deny, fuel-limited, memory-limited, all three properties proven by tests that actually exercise them.
- `veloura plugin validate`/`registry-add`/`registry-list`/`registry-set-status` CLI commands.
- No real connector-backed plugin exists to install (Milestone F hadn't shipped yet when this was built) — infrastructure only.

## Milestone G — notcurses TUI and local-only downloads/cache (PR #6)

- **Part 1 (Rust):** a `pinned` flag on `user_state` (cache-eviction exemption), `PrivacyService` cache breakdown/quota/enforcement, an `<data_dir>/api-port` discovery file, new `local-api` routes (item pin, cache status/quota/enforce, home/continue, privacy status/verify), and CLI/GUI parity.
- **Part 2 (C++20 + notcurses):** a new top-level `tui/` — a real terminal client (`veloura-tui`) talking to `local-api` over loopback HTTP, with Home/Library/Item Detail/Collections/Downloads-Cache/Privacy/Diagnostics views.
- "Downloads/offline" is local-only this milestone (pin + cache quota) — there were no connectors yet for anything to download from.

## Milestone E — Privacy and reliability (PR #5)

- AES-256-GCM field encryption for notes/private tags, keyed by an Argon2-derived key from the profile password.
- `ItemService::delete` with a `DeletionReport` confirming exactly what was removed.
- `DiagnosticsService::support_bundle` — a redacted, aggregate-only diagnostic snapshot (never titles, paths, tags, or notes).
- `Database::backup_to`/`restore_from` via rusqlite's native backup API.

## Milestone D — Desktop GUI (PR #4)

- The `crates/gui` desktop app (iced 0.13): Onboarding, Home, Library, Viewer, Privacy Center, Settings screens.

## Milestone C — Media experience (PR #3)

- FFmpeg-based video/audio probing, CBZ comic reading, a Markdown/plain-text story reader, and playback-progress tracking across media types.

## Milestone B — Local library (PR #2)

- Filesystem scanning, local full-text search (SQLite FTS5), thumbnails, favorites, and manual collections.

## Milestone A — Foundation (PR #1)

- The Rust workspace (`domain`, `database`, `application`, `local-api`, `cli`), SQLite schema with a migration runner, a loopback-only bearer-token-authenticated local API, and a CLI skeleton.
