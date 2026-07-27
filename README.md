# Veloura

Veloura is a private, local-first media browser, library, and player: a
single, consistent interface for legally accessible adult media (video,
images, stories, audio, manga/comics, galleries) across local files and
user-enabled external sources.

Full product, design, and architecture documentation lives in [`docs/`](docs/);
start with [`docs/01-product-vision.md`](docs/01-product-vision.md) and
[`docs/49-documentation-index.md`](docs/49-documentation-index.md).

## Status

Implementation has just started. This tree currently covers **Milestone A —
Foundation** from [`docs/46-implementation-plan.md`](docs/46-implementation-plan.md):
repository/build setup, domain types, SQLite storage, a local API skeleton,
and a CLI skeleton. Folder scanning, playback, the desktop GUI, connectors,
the notcurses TUI, and plugins are later milestones and are not implemented
yet.

## Repository layout

```text
crates/
├── domain/        # entities and pure domain logic (no I/O)
├── database/       # SQLite schema, migrations, FTS5
├── application/     # thin services wiring domain + database
├── local-api/        # loopback-only HTTP API (used by CLI/GUI/TUI)
└── cli/               # the `veloura` command-line binary
migrations/            # versioned SQL migration files
```

Later milestones add `crates/media`, `crates/search`,
`crates/connectors/*`, `crates/plugin-host`, `crates/gui`, and a top-level
`tui/` (C++20 + notcurses) — see
[`docs/46-implementation-plan.md`](docs/46-implementation-plan.md) for the
full sequence.

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
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`docs/27-repository-and-contributing.md`](docs/27-repository-and-contributing.md).

## Security

See [`SECURITY.md`](SECURITY.md).
