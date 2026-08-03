# veloura-tui

A C++20 + notcurses terminal client for Veloura, per `docs/09-terminal-ui.md`.
It talks to `crates/local-api` over loopback HTTP only — it never opens the
database, links `crates/application`, or runs source connectors directly (see
`docs/12-system-architecture.md`'s "notcurses TUI boundary").

## Scope this milestone

Tier B (Unicode/color) and Tier C (text-only) capability only — Tier A
(Kitty/Sixel inline bitmap thumbnails) is deferred, so cards never try to
decode or blit an image. Discover (F8) and Sources (F7) views are built and
talk to real connector data; a Queue view never shipped as a separate
concept — downloads (including queueing) live in the Downloads view (F9).
There is no Settings view (no meaningful terminal settings exist yet), and
neither is the command palette / collection-picker / shortcut-help overlay
pile — search is a plain inline text field, and `?` shows a static keybinding
list instead. Item deletion and clear-history aren't exposed either:
`local-api` has no HTTP route for them yet (the GUI/CLI call
`ItemService`/`UserStateService` in-process instead), so there's nothing for
the TUI to call.

## Building

```bash
../scripts/install-tui-deps.sh   # once, if the packages below aren't installed
cmake --preset default -S . -B ../build/tui
cmake --build ../build/tui
```

Requires notcurses 3.x (`notcurses-core`), libcurl, nlohmann-json, CMake
3.21+, and a C++20 compiler — see `docs/45-required-packages-dependencies.md`.

## Running

Start `veloura-local-api` first (it writes `<data_dir>/api-token` and
`<data_dir>/api-port`, which the TUI reads to authenticate and find the
port):

```bash
cargo run -p local-api &
../build/tui/veloura-tui
```

Set `VELOURA_TUI_PLAYER` to override the external player used for
video/audio items (defaults to `xdg-open`).

## Keybindings

```
F1  Home            F2  Library         F3  Collections
F4  Cache           F5  Privacy         F6  Diagnostics       F7  Sources
F8  Discover        F9  Downloads
j/k or Up/Down  navigate     Enter/Space  open or select     Esc  back/cancel
/   search (Library)          f  favorite (Item Detail)      p  pin (Item Detail)
c   add to collection (Item Detail)      Ctrl+L  lock        ?  help      Q  quit
```

## A note on testing this in headless/CI sandboxes

`notcurses_core_init()` sends terminal capability-interrogation escape
sequences and blocks waiting for a response. In a `tmux` session with no
real terminal emulator attached (no display, nothing answering those
queries), this call hangs indefinitely — confirmed with a 10-line
reproduction independent of any Veloura code. This isn't a bug in
`veloura-tui`; it's a property of any notcurses application run the same
way. Manual interactive verification (navigation, resize across width
tiers, lock screen, Ctrl+C/kill terminal restoration) needs a real
terminal emulator, not just a pty.
