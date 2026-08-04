# Known Issues

An honest list of what's explicitly out of scope so far, compiled from each milestone's actual scope decisions (see `CHANGELOG.md` and the linked PRs for full context). Nothing here is a bug — these are deliberate descoping decisions, most made explicitly with the user during planning.

## Connectors

- Only 4 of the 12 connector types `docs/46-implementation-plan.md`'s Workstream 10 names as "implement first" exist: a local-filesystem wrapper, an RSS/Atom feed connector, a generic Danbooru/Gelbooru-compatible booru connector (Milestone K — one `flavor`-configured implementation covers both the Danbooru-family and Gelbooru-family line items, not two separate connectors), and an OPDS catalog connector (Milestone L). Stash, Hydrus, Jellyfin, Kavita, Komga, and a metadata-only Stash-box connector are all still unbuilt — most need real service accounts/API keys unavailable in the environment this was built in (OPDS was the exception: an open, commonly-anonymous catalog standard, fully testable against fixtures). A browser-handoff template is also unbuilt. `FeedConnector`, `BooruConnector`, and `OpdsConnector` are the only download-capable connectors; the local-filesystem connector's items are already local, and none of the remaining unbuilt connector types can exercise downloads until they exist.
- `OpdsConnector` (Milestone L) supports OPDS 1.x (Atom+XML) only — OPDS 2.0 (a JSON-based successor format) is unimplemented. It has no server-side `search()` (OPDS's OpenSearch description-document mechanism for discovering a catalog's search endpoint isn't implemented, so `capabilities().search` is `false` and every query falls back to `SourceService::browse`'s local-filtering path), no facet support, and no way to reach an item nested inside a navigation sub-feed via `get_item` (only entries present in the root catalog feed resolve — items found via a prior `get_gallery` call have no id-based lookup, matching the shape of `BooruConnector`'s Gelbooru-by-id gap). Authentication is HTTP Basic only (`username`/`password` in plain `configuration_json`, same no-credential-manager caveat as `BooruConnector`'s `api_key`) — OAuth-protected OPDS servers aren't supported, and `AuthMethod` has no dedicated "Basic auth" variant, so this connector's `capabilities().authentication` reuses `ApiToken` for it (documented in `connectors::opds`'s module doc).
- Discover (`POST /api/v1/discover`) has no cross-source relevance ranking — sources are listed in a stable but not relevance-merged order — and no duplicate collapsing, so the same underlying media appearing via two connectors shows as two separate hits. There's also no true cross-source pagination: `limit_per_source` caps each source's contribution independently rather than offering a single aggregate offset/next-page token, since sources use fundamentally different pagination primitives (local search has real limit/offset, `FeedConnector` takes an opaque cursor it doesn't yet use, other connectors may use yet another mode).
- `BooruConnector`'s unsupported-query reporting is coarser than the browse-only-connector path above: `SourceService::browse` only backfills a per-clause `unsupported_clauses` report for connectors with `capabilities().search == false` (see `split_query`/`filter_locally` in `crates/application/src/source.rs`). A search-capable connector like `BooruConnector` has no side channel to report which clauses it dropped, so it instead rejects the whole query as `ConnectorResult::UnsupportedQuery` the moment any clause isn't a free-text term or a `tag:`/`-tag:` equality filter — an all-or-nothing outcome rather than "here's what was ignored." This gap was invisible until now because no search-capable non-local connector existed before this milestone.
- `BooruConnector` has no pools/galleries support (Danbooru has pools; Gelbooru's DAPI does not, and `ConnectorCapabilities` has no field to declare pools support per-flavor anyway) — `get_gallery` stays at its default `UnsupportedCapability` for both flavors.
- `BooruConnector`'s optional API key is stored directly in plain `configuration_json`, the same pattern `FeedConnector` uses for its URL — there is still no OS-credential-manager adapter in this codebase (`domain::Source.credential_ref` remains unused by every connector).

## Downloads (Milestone J, Workstream 11)

- No persistent background daemon — the CLI is still one-shot and blocks the invoking process for `download add`/`resume`; only `local-api` and the GUI can run a download in the background, and both still require a restart of that process to pick anything back up (there's no daemon surviving an app close to avoid needing one). Within that constraint, `local-api`/the GUI now do auto-resume `Queued` and crash-recovered `Paused` rows at startup and on a periodic recheck, and a real `max_concurrent_downloads` cap (`SettingsService`, default 3) gates concurrent transfers via a shared semaphore — see the changelog entry "Download crash recovery and a concurrency cap".
- Checksum verification is architecturally complete (the field, the comparison, and dedicated tests all exist) but not exercisable end-to-end in real use: RSS/Atom has no standard checksum field, so a `FeedConnector`-sourced download's `checksum_state` always lands at `Unavailable` (a locally computed `blake3` hash is still stored for future dedup use). Danbooru and Gelbooru posts both do expose an MD5 hash server-side, but `BooruConnector` deliberately doesn't surface it: `DownloadService::run_inner` always verifies with `blake3` and never consults `checksum_algorithm` to pick a hasher, so a naively-wired 32-character MD5 value would compare against a 64-character blake3 digest and fail every single download's verification. Making this safe needs an algorithm-aware `run_inner`, a new `md5` dependency, and a `RemoteItem` field addition (which breaks every existing struct-literal call site, since `RemoteItem` has no `Default` impl) — out of scope for Milestone K. A future hash-addressed connector (e.g. Hydrus), or a follow-up milestone that makes verification algorithm-aware, would be the first to light up `Verified`/`Mismatch` for real traffic.
- The default naming template has no `{creator}`/`{series}` tokens — connector-imported items don't carry that metadata yet.
- Single-variant downloads only — no gallery/multi-file/multi-chapter batch download, and `{sequence}` is always `1`.
- No progress event stream — the GUI and TUI poll on a timer rather than subscribing to push updates, the same limitation already noted below for Discover (`local-api` has no SSE/WebSocket support at all).
- Quota eviction is oldest-`completed_at`-first only — `docs/17-downloads-cache-storage.md`'s other listed policies (least-recently-opened, oldest-unviewed, per-source limits) aren't implemented, matching `enforce_cache_quota`'s existing single-policy scope.
- No encrypted downloads directory or opaque file naming option, and no separate "cache vs. permanent download" storage class beyond the `pinned` flag — every completed download is a permanent file under `<data_dir>/downloads/`.

## Content safety and blocking

- `domain::BlockRule` is consulted in one place — `DownloadService`'s
  eligibility check (Milestone J, "not blocked by an enabled block rule").
  CRUD now exists via `local-api` (`GET/POST /api/v1/block-rules`,
  `DELETE /api/v1/block-rules/:id`, `POST .../enable`, `POST .../disable`,
  see `application::BlockRuleService`/`crates/local-api/src/routes/block_rules.rs`),
  the CLI (`veloura block-rule list/add/remove/enable/disable`,
  `crates/cli/src/commands/block_rule.rs`), a GUI screen
  (`crates/gui/src/screens/block_rules.rs`), and a TUI view
  (`tui/src/views/block_rules_view.h`/`.cpp`, `F10`) — so a rule can now be
  managed from the CLI, GUI, or TUI. `docs/21-content-safety-and-compliance.md`'s
  broader user-facing block/review controls (a review queue, per-hit
  blocked/allowed decisions surfaced in the UI, etc.) still describe a
  system that isn't built yet — only the CRUD surface named above exists.
- The shipped `SafetyStatus` enum (`domain::media_item`) is a simpler
  4-value model (`Unreviewed`/`Approved`/`Flagged`/`Blocked`) than the
  7-value model `docs/21-content-safety-and-compliance.md` describes.

## Plugins

- `crates/plugin-host`'s manifest/permission/registry/sandbox infrastructure exists, but there is no real third-party plugin to install — it was built before Milestone F's connectors existed, so "preview a sandboxed third-party connector" has nothing real to preview yet.
- No real package signing or publisher PKI — the local plugin registry (`crates/plugin-host::registry`) has no signature verification or revocation-list fetching, since there's no distribution server to fetch either from.
- `plugin_host::LocalApiScope` (a plugin's requested local-API permission scopes) is defined but **unenforced** — `local-api` issues one all-or-nothing bearer token with no concept of scoped tokens yet.

## Terminal UI (`tui/`)

- Tier A (Kitty/Sixel inline bitmap thumbnails) is unimplemented — the TUI only supports Tier B (Unicode/color) and Tier C (text-only). Cards show a media-type label instead of a decoded image.
- No Settings view, and no command-palette/collection-picker/shortcut-help overlay pile — none had real backing data or meaningful terminal settings when Milestone G shipped (before connectors existed). (The Queue gap this line used to also flag is closed — see the Downloads view, F9, Milestone J.)
- Item deletion and "clear history" aren't exposed from the TUI — `local-api` has no HTTP route for either (the GUI/CLI call the relevant services in-process instead).

## GUI (`crates/gui`)

- No embedded video/audio playback — video/audio open via a configured external player, matching the CLI's behavior.
- No standalone Diagnostics screen (the Settings screen has a diagnostics panel instead). (The Downloads screen gap this line used to also flag is closed — see Milestone J.)
- Styling is functional-first, not pixel-accurate to `docs/52-sample-ui-spec.md`.
- No automated accessibility testing — no iced accessibility-testing harness exists; keyboard/contrast/reduced-motion/screen-reader gates in `docs/28-release-checklist.md`/`docs/39-release-polish-checklist.md` are unverified by automation.

## Media handling

- Comics: CBZ only — no CBR or CB7 support.
- Stories: plain text and Markdown only — no EPUB support.
- Restoring a database backup requires an app restart — there's no in-process hot-swap of the live SQLite connection after a restore.

## Release/packaging (Workstream 13)

- No real package signing — no signing keys or certificate infrastructure exist in this environment. `docs/28-release-checklist.md`'s "release is signed" and "automatic updater verifies signatures" gates are unmet.
- No packaging scripts (deb/rpm/AppImage/dmg/msi) exist yet.
- Performance tests exist only at a small representative scale, not the full `docs/22-testing-strategy.md`-documented targets (100k-item search, 1M-tag index, etc.) — see the Workstream 13 PR for what was actually measured.
- No DNS-rebinding/malicious-redirect network-attack simulation for connectors — would need a controlled-DNS test harness, judged disproportionate for the two connectors that exist today (neither does anything redirect-sensitive).
- No generic cross-connector contract-test framework — `docs/22-testing-strategy.md`'s 12-item contract suite is written for an arbitrary number of connectors; with only 2 today (one of which supports almost none of the suite, by design — feeds have no auth/pagination/search), a generic parameterized framework would be more engineering than the current connector count justifies.
- `cargo deny check advisories` has three accepted, tracked exceptions, all the same shape — an "unmaintained" flag (not a known-exploitable vulnerability) on a crate deep in `iced`'s font/rendering stack, with no safe upgrade available per the advisory itself: [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192) (`ttf-parser`), [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436) (`paste`), [RUSTSEC-2026-0206](https://rustsec.org/advisories/RUSTSEC-2026-0206) (`rustybuzz`). None are fixable by a version bump; replacing `iced` is out of scope. See `deny.toml`'s `[advisories]` section for the ignore entries and reasoning.
