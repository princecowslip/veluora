# Terminal UI Specification — notcurses

## Technology decision

The Veloura TUI uses **notcurses 3.x** through its native C API from a thin **C++20** presentation client.

The TUI is a separate executable:

```text
veloura-tui
```

It connects to the same local application service used by the GUI and CLI. It does not open the SQLite database, invoke source connectors, or access credentials directly.

```text
┌────────────────────────────────────────────┐
│ veloura-tui                                  │
│ C++20 + notcurses                          │
├────────────────────────────────────────────┤
│ TUI state, layout, input, rendering        │
│ API client, event stream, view models      │
└───────────────────┬────────────────────────┘
                    │ authenticated local IPC/API
┌───────────────────▼────────────────────────┐
│ Veloura service                  │
│ Search, library, sources, privacy, safety  │
│ downloads, progress, collections           │
└────────────────────────────────────────────┘
```

## Why notcurses

notcurses is selected because it provides:

- Native Unicode and extended grapheme handling
- 24-bit color with terminal capability fallback
- Thread-aware rendering
- Planes and piles for composable layouts
- Mouse and rich keyboard input
- Kitty, Sixel, and terminal bitmap support
- Widgets such as menus, selectors, readers, trees, reels, and progress bars
- Full-screen TUI mode and scrolling direct mode

The TUI must still remain functional when bitmap graphics are unavailable.

## Supported platforms

Primary:

- Linux
- macOS
- FreeBSD

Supported with additional testing:

- Windows 10 or later using native notcurses builds
- Windows Terminal
- WSL

The first release should treat Linux and macOS as Tier 1. Native Windows packaging is Tier 2 until CI, terminal interrogation, DLL packaging, and input behavior are proven.

## Terminal capability tiers

### Tier A — full graphics

Examples include compatible Kitty, WezTerm, foot, and Sixel-capable terminals.

Not yet implemented — the shipped TUI supports Tier B and Tier C only, so
cards never attempt to decode or blit an image today.

Features:

- Image thumbnails
- Poster previews
- Cover art
- Inline progress visuals
- 24-bit color
- Extended keyboard protocols

### Tier B — Unicode graphics

Features:

- Unicode block and sextant previews
- 24-bit or 256-color presentation
- Full navigation and metadata
- No native bitmap protocol required

### Tier C — text only

Features:

- Text rows
- ASCII separators
- Media-type symbols
- No thumbnail decoding
- Monochrome option
- Complete keyboard operation

The application detects capabilities and shows the selected tier in Diagnostics.

## Process lifecycle

1. Parse command-line options.
2. Load non-sensitive TUI preferences.
3. Connect to the Veloura local service.
4. Authenticate through the protected local token or IPC channel.
5. Initialize notcurses.
6. Probe terminal capabilities.
7. Build root planes.
8. Subscribe to application events.
9. Enter the input/render loop.
10. Stop event workers.
11. Call `notcurses_stop()` on every exit path.
12. Restore terminal state.

Fatal paths must restore the terminal. Signal handling should request orderly shutdown and avoid doing complex work inside the signal handler.

## notcurses initialization

Recommended options:

```cpp
notcurses_options options{};
options.flags =
    NCOPTION_SUPPRESS_BANNERS |
    NCOPTION_NO_QUIT_SIGHANDLERS;
options.loglevel = NCLOGLEVEL_WARNING;

notcurses* nc = notcurses_init(&options, stdout);
```

The application owns its shutdown policy. The exact signal-handler flags should be validated during implementation.

Optional modes:

- Alternate-screen full TUI for interactive use
- Preserved-screen mode for diagnostics
- `ncdirect` mode for selected CLI visual output

## Plane hierarchy

```text
standard plane
├── background plane
├── navigation plane
├── header plane
├── content pile
│   ├── source plane
│   ├── result plane
│   └── details plane
├── compact-player plane
├── status plane
├── overlay pile
│   ├── command palette
│   ├── collection picker
│   ├── filters
│   ├── confirmation dialog
│   └── shortcut help
└── privacy shield plane
```

Each major region owns its plane and view model.

Rules:

- Render only invalidated planes.
- Avoid recreating planes on every frame.
- Reparent or resize on terminal changes.
- Keep overlays in a distinct pile.
- Place the privacy shield above every content plane.
- Never render blocked media before policy filtering.

## Responsive layout

### Wide: 140 columns or more

```text
┌────────────────┬──────────────────────────────────┬──────────────────────┐
│ Sources/views  │ Results                          │ Details              │
│ 22–28 columns  │ flexible                         │ 36–44 columns        │
└────────────────┴──────────────────────────────────┴──────────────────────┘
```

### Standard: 90–139 columns

```text
┌────────────────────┬─────────────────────────────────────────────────────┐
│ Sources            │ Results                                             │
├────────────────────┴─────────────────────────────────────────────────────┤
│ Collapsible details or compact player                                    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Narrow: below 90 columns

- One primary pane
- Tab or key navigation between Sources, Results, and Details
- Full-screen overlays
- Compact status bar
- No inline bitmap preview by default

### Tiny: below 60 columns or 18 rows

- Text-only mode
- One-line status
- Essential actions only
- Warning that the terminal is below the recommended size

## Main views

- Home
- Library
- Collections
- Cache
- Privacy
- Diagnostics
- Sources
- Discover
- Downloads (includes queueing — there is no separate Queue view)

Settings is not yet built as a TUI view (no meaningful terminal-specific
settings exist yet).

## Home

The Home view contains:

- Continue reel
- Feed list
- Recent local additions
- Source notices
- Compact download activity

Feed tabs:

```text
All  Local  Sources  Chapters  Downloads  Notices
```

## Library

Library provides:

- Result reel or table
- Filter overlay
- Sort selector
- Media-type tabs
- Selection mode
- Detail pane
- Layout selector

TUI layouts:

- Detailed list
- Compact list
- Table
- Two-column cards
- Thumbnail reel when supported
- Source groups
- Timeline

Masonry is not used in the TUI because it weakens keyboard order and terminal resize behavior.

## Search

The search overlay uses `ncreader` or a custom single-line editor.

Features:

- Query history according to privacy settings
- Namespace autocomplete
- Source-aware suggestions
- Syntax highlighting
- Inline errors
- Saved search action
- Cancel current unified search

Example:

```text
orientation:bisexual media:video source:local -tag:user:block
```

Search results stream independently. A source status strip displays:

```text
LOCAL ✓   STASH ✓   BOORU …   FEED !rate-limit
```

## Filters

Filter overlay groups:

- Media
- Categories
- Sexual orientation
- Participant composition
- Acts and themes
- Source
- Technical
- Progress
- Visibility and blur

Sensitive field names may be replaced with user-defined neutral labels.

## Input model

### Global

```text
Ctrl+P         Command palette (Ctrl+K is avoided here since it is a reserved readline/terminal binding for "kill to end of line"; the GUI uses Ctrl/Cmd+K for the same command palette)
/              Search
?              Help
Esc            Close overlay, back, or cancel
Ctrl+L         Lock
Ctrl+Shift+P   Start or end private session
Tab            Next pane
Shift+Tab      Previous pane
F1             Home
F2             Library
F3             Collections
F4             Cache
F5             Privacy
F6             Diagnostics
F7             Sources
F8             Discover
F9             Downloads
```

### Navigation

```text
j / Down       Next item
k / Up         Previous item
h / Left       Parent, previous page, or previous pane
l / Right      Open, next page, or next pane
g              First item
G              Last item
Ctrl+F         Page down
Ctrl+B         Page up
Enter          Open
```

### Item actions

```text
Space          Play or pause
r              Resume or read
e              Open externally
f              Favorite
q              Add to queue
c              Collection picker
d              Download when allowed
i              Details
t              Edit local tags
b              Block menu
y              Copy canonical URL
v              Change layout or viewer mode
z              Toggle blur for selected item
```

Keys are configurable. A text input consumes printable keys before global shortcuts.

## Mouse input

Optional mouse support includes:

- Select row
- Activate button
- Scroll reel
- Resize split panes
- Open context menu
- Seek compact player

All mouse actions must have keyboard equivalents.

## Color mapping

Use the Veloura tokens:

- Indigo: selection and primary actions
- Lavender: focus and soft highlight
- Iris/violet: active secondary emphasis
- Moonstone: source information
- Mint: ready and complete
- Seafoam/aquamarine: playback and progress
- Yellow: warning and pending
- Red: blocked, failed, and destructive

Monochrome mode uses labels:

```text
[OK] [INFO] [WAIT] [WARN] [ERROR] [BLOCKED]
```

## Bitmap and media previews

notcurses visuals are optional presentation enhancements.

Preview pipeline:

1. Request thumbnail metadata from the local service.
2. Receive a local cache path or approved temporary file.
3. Validate that the path belongs to the TUI thumbnail cache.
4. Load with `ncvisual_from_file()`.
5. Render using the best available blitter.
6. Fall back to Unicode or text on failure.
7. Destroy the visual after eviction.

The TUI must not receive remote credentials or signed URLs solely to produce a preview.

### Preview privacy

- Locked mode destroys or covers preview planes.
- Public sources default to strong blur or solid placeholders.
- Policy-blocked items never load a visual.
- Blur may be approximated with a pre-generated blurred thumbnail.
- Terminal scrollback and title updates must not expose sensitive titles.

## Playback

The TUI does not decode full media.

It controls:

- Built-in application playback through the local service
- An external player such as mpv
- A remote personal-server playback session

Compact player shows:

- State
- Title according to privacy mode
- Position
- Duration
- Queue position
- Device
- Error or reconnect state

## Event loop

Use one render thread and background workers.

```text
main thread:
  notcurses input
  state reduction
  plane invalidation
  notcurses_render()

API worker:
  requests
  event stream
  reconnect and backoff

thumbnail worker:
  cache lookup
  thumbnail decode preparation
```

Workers send typed messages to the main thread. No worker calls notcurses rendering APIs unless the chosen API is explicitly documented as safe for that operation.

## Performance rules

- Target 30 FPS only during active visual changes.
- Idle views render only after state or input changes.
- Coalesce progress events.
- Debounce resize events.
- Bound thumbnail decode concurrency.
- Do not animate hidden planes.
- Pause nonessential animation over SSH.
- Disable bitmap previews on high-latency sessions by default.

## Privacy mode

Private mode:

- Replaces titles with neutral item identifiers.
- Hides tags and descriptions.
- Disables terminal-title updates.
- Uses an alternate screen.
- Clears or covers all planes before exit.
- Disables persistent search history.
- Prevents source names appearing in shell completion.
- Avoids copying explicit text unless the user confirms.

## Accessibility

- Monochrome mode
- Text labels for every status
- Configurable keys
- No meaning carried by color alone
- Stable focus order
- Plain list mode
- Adjustable density
- Disable previews and animations
- High-contrast focus
- Screen-reader-friendly non-alternate-screen mode where practical

## Testing

Required TUI tests:

- Plane layout at all width tiers
- Resize during search
- Lock while preview is visible
- Terminal disconnect
- Service restart and reconnect
- Unicode grapheme rendering
- RTL and BiDi metadata
- Mouse disabled
- Monochrome terminal
- 16-color terminal
- True-color terminal
- Kitty graphics
- Sixel graphics
- SSH text-only mode
- Windows Terminal
- WSL
- `TERM` unset or invalid
- Every fatal and normal exit restores the terminal
