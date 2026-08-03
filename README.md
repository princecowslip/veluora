# Veloura

Veloura is a private, local-first media browser, library, and player: a
single, consistent interface for legally accessible adult media (video,
images, stories, audio, manga/comics, galleries) across local files and
user-enabled external sources.

Full product, design, and architecture documentation lives in [`docs/`](docs/);
start with [`docs/01-product-vision.md`](docs/01-product-vision.md) and
[`docs/49-documentation-index.md`](docs/49-documentation-index.md).

## Status

This tree covers Milestones A through L from
[`docs/46-implementation-plan.md`](docs/46-implementation-plan.md); see
[`CHANGELOG.md`](CHANGELOG.md) for the full milestone-by-milestone history
and [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) for explicitly out-of-scope gaps:

- **Milestone A — Foundation**: repository/build setup, domain types,
  SQLite storage, a local API skeleton, and a CLI skeleton.
- **Milestone B — Local library**: folder scanning, local search,
  metadata, thumbnails, favorites, and collections.
- **Milestone C — Media experience**: video/audio/comic/story probing
  and thumbnailing (via FFmpeg and CBZ archive reading), playback
  progress tracking, and an external-player launch path.
- **Milestone D — Desktop MVP**: a functional-first desktop GUI
  (`crates/gui`, built on [iced](https://iced.rs)) covering Onboarding,
  Home, Library, Viewer, Privacy Center, and Settings.
- **Milestone E — Privacy and reliability**: field-level AES-256-GCM
  encryption for notes/private tags, verified deletion, and diagnostics.
- **Milestones F, H, K, L — Connectors and plugins**: an async `Connector`
  trait with local filesystem, RSS/Atom feed, booru (Danbooru/Gelbooru),
  and OPDS connectors (`crates/connectors`), plus a `wasmtime`-sandboxed
  plugin host (`crates/plugin-host`) with a manifest/permission model.
- **Milestone G — Terminal UI**: a real notcurses terminal client
  (`tui/`) talking to `local-api` over loopback HTTP.
- **Milestones I, J — Discover and downloads**: unified cross-source
  search (`veloura discover`, GUI/TUI Discover screens) and a full
  download manager (queueing, resume, quota eviction) across CLI,
  `local-api`, GUI, and TUI.

Only `crates/search` (a dedicated search crate, as opposed to the search
already built into `application`/`database`) remains unbuilt.

## Repository layout

```text
crates/
├── domain/          # entities and pure domain logic (no I/O)
├── database/        # SQLite schema, migrations, FTS5
├── media/            # FFmpeg probing, CBZ archives, story parsing, external players
├── application/       # services wiring domain + database + media + connectors
├── local-api/          # loopback-only HTTP API (used by CLI/GUI/TUI)
├── plugin-host/          # plugin manifest/permissions and the wasmtime sandbox
├── connectors/             # source connectors (local filesystem, feed, booru, OPDS)
├── cli/                     # the `veloura` command-line binary
└── gui/                      # the desktop GUI (iced), `veloura-gui` binary
migrations/                    # versioned SQL migration files
tui/                            # the C++20 + notcurses terminal client, veloura-tui
```

See [`docs/46-implementation-plan.md`](docs/46-implementation-plan.md) for
the full milestone sequence.

## Building

Requires a stable Rust toolchain (see `rust-toolchain.toml`).

```bash
cargo build --workspace
cargo test --workspace
```

## Running

```bash
# Local API (loopback only; prints its port and auth token on startup)
cargo run -p local-api

# CLI
cargo run -p cli -- doctor
cargo run -p cli -- doctor --output json

# Desktop GUI (links `application` directly, like the CLI — no local-api needed)
cargo run -p gui
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`docs/27-repository-and-contributing.md`](docs/27-repository-and-contributing.md).

## Security

See [`SECURITY.md`](SECURITY.md).
