# Changelog

Format loosely follows [Keep a Changelog](https://keepachangelog.com/). Veloura hasn't cut a versioned release yet — this tracks what each milestone actually shipped. See `KNOWN_ISSUES.md` for what's explicitly out of scope so far.

## Milestone J — Downloads and offline use (Workstream 11)

- `FeedConnector` now declares `capabilities().downloads = true` and extracts a per-entry downloadable attachment (an RSS `<enclosure>` or an Atom `rel="enclosure"` link) into new `RemoteItem` fields (`download_url`/`download_mime_type`/`download_size_bytes`) — the first connector able to exercise Workstream 11's download machinery. `SourceService::import_remote_item` now sets `media_variants.download_permitted`/`remote_url`/`mime_type`/`file_size` from these fields instead of always pointing a variant at the item's canonical webpage.
- New `application::download::DownloadService`: eligibility checks (download-permitted, not already local, source enabled and downloads-capable, not blocked by an enabled block rule, writable destination, quota pre-check), a naming-template-driven destination, and a fetch/resume/verify/finalize engine. Pause/resume/cancel are coordinated through the shared SQLite `downloads` row (an atomic claim plus a polled `state` column) rather than an in-memory flag, since neither the CLI (one-shot) nor `local-api`/GUI (separate long-lived processes) share memory — a `download pause` issued from a completely independent process genuinely stops a transfer in flight.
- Downloads stream straight to a `<data_dir>/temp/downloads/<id>.part` temp file and are only ever moved into `<data_dir>/downloads/` via an atomic rename after the transfer (and, when the source declared one, a `blake3` checksum) succeeds — partial or corrupt bytes never appear at the final path. Retried/resumed fetches send `Range`+`If-Range`; a `206` response appends, any other response (server ignored the range, or the content changed) restarts from byte zero rather than corrupting the file with a stale-plus-new splice.
- `PrivacyService::enforce_download_quota` (paired with `download_directory_size_bytes`) extends the existing cache-quota machinery — factored into a shared `evict_until_under_quota` helper both now call — to evict completed, non-pinned download files oldest-`completed_at`-first. A download is protected by either its own `pinned` flag or its item's existing `user_state.pinned` flag, so pinning an item anywhere also protects its downloads.
- `migrations/0006_downloads.sql` extends the `downloads` table (unused scaffolding since Milestone A) with `source_id`/`pinned`/`temp_path`/`expected_checksum`/`checksum_algorithm`/`etag`/`last_modified`/`updated_at` — its first real consumer.
- New `local-api` routes (`GET/POST /api/v1/downloads`, per-id `pause`/`resume`/`cancel`/`pin`/`DELETE`, `eligibility`, `quota`, `status`, `enforce-quota`), `veloura download add/list/pause/resume/cancel/remove/pin/eligibility/quota/enforce-quota` CLI commands, a GUI Downloads screen (with a contextual "Download" button on the Viewer screen), and a TUI Downloads view (F9) — closing the "No Queue view exists yet" gap `KNOWN_ISSUES.md` flagged for the terminal client. `local-api`'s `add`/`resume` spawn the fetch on its own runtime and return `202` immediately, since it's the only long-lived surface that can run a download to completion in the background; the CLI's `add`/`resume` block the invoking process (no daemon exists), matching `source health-check`'s existing "a non-ideal outcome is still `Success`" exit-code convention.

## Milestone I — Unified Discover (cross-source search) (PR #12)

- New `application::discover::DiscoverService`: aggregates the local library (always searched, even with no "Local filesystem" source configured) with every enabled, non-local connector source in one call. Per-source failures and unsupported query clauses are isolated and reported (`DiscoverSourceStatus`) rather than aborting the whole aggregate, and each hit (`DiscoverHit`) reports whether it's already in the local library.
- New `POST /api/v1/discover` (local-api), `veloura discover <query>` (CLI), a GUI Discover screen, and a TUI Discover view (F8) — closing the gap `KNOWN_ISSUES.md` flagged three times, across the Connectors, TUI, and GUI sections.
- No unified pagination across heterogeneous sources this milestone (local search offset/limit vs. a connector's opaque cursor) — `limit_per_source` caps each source's contribution independently, documented as a known limitation in `KNOWN_ISSUES.md`.

### TUI Sources view

- New `tui/` Sources view (F7): list, add (local filesystem or RSS/Atom feed), enable/disable, remove, health-check, browse, and import a browsed item into the local library — the terminal counterpart of the GUI Sources screen and `veloura source ...`.

### GUI Sources management screen

- New `crates/gui` Sources screen: add a source (local filesystem or RSS/Atom feed), enable/disable, remove, health-check, browse, and import a browsed item into the local library — closing the gap left by Milestone F's connectors shipping after Milestone D's GUI.

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
