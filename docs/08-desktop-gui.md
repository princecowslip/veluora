# Desktop GUI Specification

## Application shell

### Wide layout

```text
┌──────────────┬──────────────────────────────────────────┐
│ Navigation   │ Top bar: search, commands, privacy      │
│              ├──────────────────────────────────────────┤
│ Home         │ Main content                             │
│ Library      │                                          │
│ Discover     │                                          │
│ Collections  │                                          │
│ Downloads    │                                          │
│ Settings     │                                          │
│              ├──────────────────────────────────────────┤
│ Player       │ Compact playback or reading status       │
└──────────────┴──────────────────────────────────────────┘
```

### Narrow layout

- Navigation becomes a bottom bar or drawer.
- Filters open as a full-height sheet.
- Item details become a separate route.
- Compact player stays above the bottom navigation.

## Home screen

### Header

- Greeting can be disabled.
- Lock or private-session state is always visible.
- Global search is the most prominent control.

### Sections

- Continue
- Queue
- Personalized feed
- Recently added
- Pinned collections
- Saved searches
- Source status
- Storage and indexing activity
- Optional local recommendations

Users can reorder or hide sections. See [05 — Information Architecture](05-information-architecture.md) for how the Personalized feed is composed.

## Library screen

### Toolbar

- Media type tabs
- Search
- Filter
- Sort
- View mode
- Selection mode
- New collection

### Grid card

A card can display:

- Thumbnail
- Media type
- Duration or page count
- Progress
- Favorite state
- Source badge
- Local or remote state

Hover or focus reveals secondary actions. Touch mode uses an overflow button.

### Detail pane

On wide screens, optional split-view detail includes:

- Larger preview
- Primary action
- Description
- Tags
- Progress
- Technical information
- Collection actions

## Discover screen

### Source selector

Modes:

- All enabled
- Selected sources
- One source
- Local only

### Search status

Show one row per source with:

- Result count
- Completion status
- Rate-limit state
- Authentication state
- Retry action

### Result grouping

User options:

- Unified ranking
- Group by source
- Group by media type
- Collapse duplicates

## Player window

### Video layout

- Main video surface
- Auto-hiding controls
- Optional queue sidebar
- Metadata drawer
- Subtitle and quality menus
- Fullscreen and picture-in-picture controls

### Image layout

- Centered image surface
- Zoom and pan
- Filmstrip
- Metadata drawer
- Slideshow controls

### Story layout

- Reading column
- Chapter sidebar
- Typography toolbar
- Search-in-story
- Progress marker
- Distraction-free mode

### Manga and comic layout

- Page or strip
- Reading-direction control
- Page scrubber
- Chapter list
- Fit and crop controls
- Double-page mode

## Collections

### Manual collection

- Cover
- Title and description
- Sort mode
- Drag-to-reorder
- Multi-select edit
- Export

### Smart collection

- Saved query
- Live preview
- Refresh status
- Query explanation

## Downloads

Use a table or stacked rows with:

- Item
- Source
- Type: cache or permanent
- Progress
- Speed
- Remaining size
- State
- Actions

Bulk actions:

- Pause
- Resume
- Cancel
- Retry
- Clear completed
- Move storage location where supported

## Source setup wizard

1. Select source type.
2. Review capabilities.
3. Review permissions.
4. Authenticate.
5. Test connection.
6. Choose browsing defaults.
7. Choose cache and download behavior.
8. Finish with a source health summary.

## Settings design

Settings should use searchable categories with immediate validation.

Sensitive settings such as remote access, browser-cookie import, and plugin filesystem access should include risk explanations and require explicit confirmation.

## Window behavior

- Remember position and size per display.
- Support fullscreen viewer windows.
- Optionally detach player.
- Prevent explicit thumbnails from appearing in operating-system recent-window previews when neutral mode is enabled.
- Offer startup locked state.
