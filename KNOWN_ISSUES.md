# Known Issues

An honest list of what's explicitly out of scope so far, compiled from each milestone's actual scope decisions (see `CHANGELOG.md` and the linked PRs for full context). Nothing here is a bug — these are deliberate descoping decisions, most made explicitly with the user during planning.

## Connectors

- Only 2 of the 12 connector types `docs/46-implementation-plan.md`'s Workstream 10 names as "implement first" exist: a local-filesystem wrapper and an RSS/Atom feed connector. OPDS, Stash, Hydrus, Jellyfin, Kavita, Komga, a Danbooru-family connector, a Gelbooru-family connector, a metadata-only Stash-box connector, and a browser-handoff template are all unbuilt — most need real service accounts/API keys unavailable in the environment this was built in.
- Neither existing connector declares the `downloads` capability, so Workstream 11 ("Downloads and offline use" — eligibility checks, a download queue, pause/resume, verification, quotas) has nothing to actually exercise it. What exists instead (Milestone G) is local-only: a `pinned` cache-eviction-exemption flag and cache-quota enforcement over already-local files.
- No unified cross-source search — `POST /search` stays local-library-only; browsing a connector-backed source is a separate, explicit action (`/sources/:id/browse`), not merged into search results.

## Plugins

- `crates/plugin-host`'s manifest/permission/registry/sandbox infrastructure exists, but there is no real third-party plugin to install — it was built before Milestone F's connectors existed, so "preview a sandboxed third-party connector" has nothing real to preview yet.
- No real package signing or publisher PKI — the local plugin registry (`crates/plugin-host::registry`) has no signature verification or revocation-list fetching, since there's no distribution server to fetch either from.
- `plugin_host::LocalApiScope` (a plugin's requested local-API permission scopes) is defined but **unenforced** — `local-api` issues one all-or-nothing bearer token with no concept of scoped tokens yet.

## Terminal UI (`tui/`)

- Tier A (Kitty/Sixel inline bitmap thumbnails) is unimplemented — the TUI only supports Tier B (Unicode/color) and Tier C (text-only). Cards show a media-type label instead of a decoded image.
- No Discover, Sources, Queue, or Settings views, and no command-palette/collection-picker/shortcut-help overlay pile — none had real backing data or meaningful terminal settings when Milestone G shipped (before connectors existed).
- Item deletion and "clear history" aren't exposed from the TUI — `local-api` has no HTTP route for either (the GUI/CLI call the relevant services in-process instead).

## GUI (`crates/gui`)

- No Sources management screen — connectors (Milestone F) shipped after the GUI (Milestone D) did, so there's no UI for configuring a connector-backed source yet, only CLI/local-api.
- No embedded video/audio playback — video/audio open via a configured external player, matching the CLI's behavior.
- No Discover, Downloads, or standalone Diagnostics screen (the Settings screen has a diagnostics panel instead).
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
