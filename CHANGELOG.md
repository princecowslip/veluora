# Changelog

Format loosely follows [Keep a Changelog](https://keepachangelog.com/). Veloura hasn't cut a versioned release yet — this tracks what each milestone actually shipped. See `KNOWN_ISSUES.md` for what's explicitly out of scope so far.

## Block rule CRUD

- New `application::BlockRuleService` (create/list/remove/set_enabled) is the first code to write to the `block_rules` table — previously `DownloadService::is_blocked` was the only consumer, and there was no way to create, list, or remove a rule anywhere (see `KNOWN_ISSUES.md`).
- New `local-api` routes (`GET/POST /api/v1/block-rules`, `DELETE /api/v1/block-rules/:id`, `POST .../enable`, `POST .../disable`) and `veloura block-rule list/add/remove/enable/disable` CLI commands, mirroring the existing `sources` resource's shape. No GUI/TUI screen yet — see `KNOWN_ISSUES.md`.
- `DownloadService::is_blocked`'s row-mapping/string-conversion helpers moved into `application::block_rules` so there's one implementation instead of two; no behavior change.

## Download crash recovery and a concurrency cap

- Fixed a real bug in `DownloadService::claim`: a `local-api`/GUI process killed mid-transfer left its row permanently stuck `Active` — `claim()` only reclaims `Queued`/`Paused`/`Failed` rows, so a fresh `run`/`resume` silently no-op'd on it forever. New `DownloadService::repair_stale_active` (time-based on the existing `updated_at` heartbeat, so it's safe even with `local-api` and the GUI running against the same database at once) and `repair_if_stale` (single-row, used by the CLI) recover these back to `Paused`.
- `local-api` and the GUI now auto-resume `Queued` and crash-recovered `Paused` rows at startup and on a 60s periodic recheck (`DownloadService::resumable_after_restart`) — user-deliberately-paused rows are left alone. This doesn't add the persistent background daemon `KNOWN_ISSUES.md` describes as unbuilt (a restart is still required to trigger recovery), but it closes the "stuck forever, resume silently no-ops" failure mode and the "must remember to manually resume everything after a crash" gap.
- New `SettingsService::max_concurrent_downloads` (default 3) backs a real concurrency cap: `local-api` (`ApiState::download_semaphore`) and the GUI (`App::download_semaphore`) gate every spawned download through a `tokio::sync::Semaphore`, so `add`/`resume` calls beyond the cap wait for a slot rather than all running unconditionally — closing `docs/37-settings-and-preferences.md`'s "Maximum concurrent downloads" gap.

## Milestone L — OPDS connector (Workstream 10)

- New `connectors::OpdsConnector`: an OPDS 1.x (Atom+XML) catalog connector for self-hosted book/comic/manga servers — Komga, Kavita, and Calibre-Web all serve OPDS (`configuration_json`: `url`, optional `username`/`password` for HTTP Basic auth). It's the fourth connector to exist and reuses `feed_rs` (already a dependency for `FeedConnector`) rather than adding a direct XML dependency, resolving relative `href`s (which real OPDS servers use extensively) against the catalog's own URL via `feed_rs`'s `base_uri` parser option.
- The first real implementation of `Connector::get_gallery` — defined since the trait's introduction but left `UnsupportedCapability` by both `FeedConnector` and `BooruConnector` (see `KNOWN_ISSUES.md`). Each entry in a fetched feed is classified as a navigation entry (a `<link>` whose `type` carries `profile=opds-catalog`, pointing at a sub-catalog feed — mapped to a `MediaType::Gallery` `RemoteItem` whose `source_item_id` is the sub-feed's own URL) or an acquisition entry (a `<link rel="http://opds-spec.org/acquisition...">`, pointing at a downloadable publication — mapped to a `MediaType::Story`/`Comic`/`Other` `RemoteItem` with real `download_url`/`download_mime_type`/`download_size_bytes`). `get_gallery` fetches a navigation entry's `source_item_id` directly, since an OPDS hierarchy is discovered by walking links, not a global id index.
- `capabilities().pagination = PaginationMode::Cursor`: `browse`'s `page` parameter, when set, is the previous page's `rel="next"` feed-level link — the first connector to give that pagination mode real (if not yet UI-wired) semantics, rather than declaring it and ignoring the parameter.
- `SourceService::registry()`, the GUI Sources screen's `ConnectorChoice` (`crates/gui/src/screens/sources.rs`), and the TUI Sources view's `AddStep` flow (`tui/src/views/sources_view.h`/`.cpp`) all updated so an OPDS source can be added end-to-end from any surface — `local-api` and the CLI needed no changes, matching Milestone K's booru-connector precedent.
- OPDS 2.0 (the JSON-based successor format), OpenSearch-based `search()`, facets, and non-Basic auth schemes are out of scope this milestone — see `KNOWN_ISSUES.md`.

## Milestone K — Booru connector (Workstream 10)

- New `connectors::BooruConnector`: a generic, `flavor`-configured connector speaking both Danbooru's REST/JSON API and Gelbooru's DAPI (`configuration_json`: `flavor` (`"danbooru"`/`"gelbooru"`), `base_url`, optional `api_key`/`login_or_user_id`) — one implementation covers both of Workstream 10's "One Danbooru-family connector"/"One Gelbooru-family connector" line items rather than two separate connector types, matching `docs/14-source-connectors.md`'s "generic booru-compatible connector" reference-implementation guidance. It's the third connector to exist (after local filesystem and RSS/Atom feed) and the second to declare `capabilities().downloads`.
- The first connector with real server-side `capabilities().search`: `search` translates free text and `tag:`/`-tag:` equality clauses into the space-joined tag query both APIs natively understand, and reports `ConnectorResult::UnsupportedQuery` for anything else (any other field, a comparison/range predicate, or an `(a OR b)` group) rather than silently dropping part of a query — see `KNOWN_ISSUES.md` for how this differs from the coarser, browse-only-connector reporting path `SourceService::browse` already had.
- Gelbooru's DAPI response envelope (bare array vs. `{"post": [...]}` vs. a single wrapped object vs. `{"post": null}`/no key for zero results) is normalized defensively rather than assuming one shape.
- A minimal per-connector self-throttle (one request/second) backs the `rate_limit` this connector declares, matching `docs/41-expanded-source-catalogue.md`'s "conservative request limits" guidance for booru sources — the first connector where `capabilities().rate_limit` reflects enforced behavior rather than being purely documentary.
- `crates/connectors/src/http_util.rs`: the response-size-cap helper `FeedConnector` introduced in Milestone I is now shared between `FeedConnector` and `BooruConnector` rather than duplicated.
- `SourceService::registry()`, the GUI Sources screen's `ConnectorChoice` (`crates/gui/src/screens/sources.rs`), and the TUI Sources view's `AddStep` flow (`tui/src/views/sources_view.h`/`.cpp`) all updated so a booru source can be added end-to-end from any surface — `local-api` and the CLI needed no changes, since both already accept an opaque `connector_id`/`configuration_json`.
- No pools/galleries support (Danbooru-only, no `ConnectorCapabilities` field for it), no checksum verification (both APIs expose an MD5 hash, but wiring it through today would make every download fail its own blake3-only verification — see `KNOWN_ISSUES.md`), and the optional API key is stored in plain `configuration_json`, matching `FeedConnector`'s existing URL-storage pattern (no OS-credential-manager adapter exists in this codebase yet).

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
