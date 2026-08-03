# CLAUDE.md

Veloura is a private, local-first media browser, library, and player: a
single interface for legally accessible adult media (video, images,
stories, audio, manga/comics, galleries) across local files and
user-enabled external sources.

## Workspace layout

A Rust workspace plus a separate C++ terminal client:

```text
crates/
├── domain/          # entities and pure domain logic (no I/O)
├── database/        # SQLite schema, migrations, FTS5
├── media/            # FFmpeg probing, CBZ archives, story parsing, external players
├── application/       # services wiring domain + database + media + connectors
├── local-api/          # loopback-only HTTP API (used by CLI/GUI/TUI)
├── plugin-host/          # plugin manifest/permissions and the wasmtime sandbox
├── connectors/             # source connectors: local filesystem, feed, booru, OPDS
├── cli/                     # the `veloura` command-line binary
└── gui/                      # the desktop GUI (iced), `veloura-gui` binary
migrations/                    # versioned SQL migration files
tui/                            # C++20 + notcurses terminal client, veloura-tui (CMake, not part of the Cargo workspace)
```

Connectors are plain modules inside the one `crates/connectors` crate
(`feed.rs`, `booru.rs`, `opds.rs`), not separate sub-crates. There is no
`crates/search` crate.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

TUI (separate CMake build, requires notcurses/libcurl/nlohmann-json — see
`docs/45-required-packages-dependencies.md`):

```bash
./scripts/install-tui-deps.sh   # once, if packages aren't installed
cmake --preset default -S tui -B build/tui
cmake --build build/tui
```

## Where to find current status

`CHANGELOG.md` (what's shipped, per milestone) and `KNOWN_ISSUES.md`
(explicit known gaps) are the two documents kept accurate and current —
check them before trusting anything in `docs/` at face value. The `docs/`
directory (53 numbered files) was written as an up-front planning package
before implementation began; most of it was never revisited as milestones
shipped, so it mixes real current-state documentation with unmarked
aspirational/target-state design. Docs with an "Implementation status"
note near the top have already been reconciled; treat anything else in
`docs/` as a design target unless you cross-check it against the
CHANGELOG. `docs/49-documentation-index.md` is the full doc index.

## Conventions

- Typed errors via `thiserror`, not string errors.
- The local API binds to loopback only and issues one all-or-nothing
  bearer token — there are no per-scope tokens yet.
- Connector configuration (including credentials) is passed as opaque
  `configuration_json` — there is no OS-credential-manager integration.
- The GUI, TUI, and CLI all call the same `application` services; the TUI
  additionally never opens the database or links `application` directly —
  it only talks to `local-api` over loopback HTTP.
