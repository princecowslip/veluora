# Known Issues

An honest list of what's explicitly out of scope so far, compiled from each milestone's actual scope decisions (see `CHANGELOG.md` and the linked PRs for full context). Nothing here is a bug — these are deliberate descoping decisions, most made explicitly with the user during planning.

## Connectors

- Only 2 of the 12 connector types `docs/46-implementation-plan.md`'s Workstream 10 names as "implement first" exist: a local-filesystem wrapper and an RSS/Atom feed connector. OPDS, Stash, Hydrus, Jellyfin, Kavita, Komga, a Danbooru-family connector, a Gelbooru-family connector, a metadata-only Stash-box connector, and a browser-handoff template are all unbuilt — most need real service accounts/API keys unavailable in the environment this was built in. `FeedConnector` is the only download-capable connector (Milestone J); the local-filesystem connector's items are already local, and none of the unbuilt connector types can exercise downloads until they exist.
- Discover (`POST /api/v1/discover`) has no cross-source relevance ranking — sources are listed in a stable but not relevance-merged order — and no duplicate collapsing, so the same underlying media appearing via two connectors shows as two separate hits. There's also no true cross-source pagination: `limit_per_source` caps each source's contribution independently rather than offering a single aggregate offset/next-page token, since sources use fundamentally different pagination primitives (local search has real limit/offset, `FeedConnector` takes an opaque cursor it doesn't yet use, other connectors may use yet another mode).

## Downloads (Milestone J, Workstream 11)

- No persistent background daemon or automatic resume-on-restart of `Queued` rows — the CLI is one-shot and blocks the invoking process for `download add`/`resume`; only `local-api` (a long-lived process) and the GUI can run a download in the background, and neither does so across an app restart. `docs/37-settings-and-preferences.md`'s "Maximum concurrent downloads" setting is unimplemented, since there's no scheduler to cap.
- Checksum verification is architecturally complete (the field, the comparison, and dedicated tests all exist) but not exercisable end-to-end in real use: RSS/Atom has no standard checksum field, so a `FeedConnector`-sourced download's `checksum_state` always lands at `Unavailable` (a locally computed `blake3` hash is still stored for future dedup use). A future hash-addressed connector (e.g. Hydrus) would be the first to light up `Verified`/`Mismatch` for real traffic.
- The default naming template has no `{creator}`/`{series}` tokens — connector-imported items don't carry that metadata yet.
- Single-variant downloads only — no gallery/multi-file/multi-chapter batch download, and `{sequence}` is always `1`.
- No progress event stream — the GUI and TUI poll on a timer rather than subscribing to push updates, the same limitation already noted below for Discover (`local-api` has no SSE/WebSocket support at all).
- Quota eviction is oldest-`completed_at`-first only — `docs/17-downloads-cache-storage.md`'s other listed policies (least-recently-opened, oldest-unviewed, per-source limits) aren't implemented, matching `enforce_cache_quota`'s existing single-policy scope.
- No encrypted downloads directory or opaque file naming option, and no separate "cache vs. permanent download" storage class beyond the `pinned` flag — every completed download is a permanent file under `<data_dir>/downloads/`.

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
