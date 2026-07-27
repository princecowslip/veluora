# Sample Mock UI Specification

## Overview

See also [32 — UI/UX Wireframes](32-ui-wireframes.md) for structural ASCII wireframes and [51 — Sample Mock UI Specification](51-sample-mock-ui-spec.md) for mock states and interactions of the same screens — the three documents are complementary, not contradictory.

This specification defines three reference screens:

- Home
- Library
- Viewer

The mock screens use a pure-black canvas, indigo primary controls, seafoam playback accents, yellow warnings, and red destructive or blocked states.

## Shared shell

### Desktop dimensions

Reference viewport:

```text
1440 × 960
```

Shell measurements:

```text
Navigation rail: 220 px
Top bar: 64 px
Content gutter: 24 px
Section gap: 32 px
Compact player: 72 px
Right detail panel: 360 px
```

### Persistent shell components

- Product mark
- Home
- Library
- Discover
- Collections
- Downloads
- Settings
- Global search
- Private-session indicator
- Source-health indicator
- Compact player

## Home screen

### Purpose

Resume activity, show the user's library feed, and expose source updates without turning Home into an uncontrolled external-content wall.

### Home feed

The feed is a mixed, configurable stream of:

- Continue items
- Recently added local media
- Queue activity
- New items from pinned sources
- New chapters from followed series
- New episodes from personal servers
- Saved-search matches
- Download completions
- Source-status notices

External source posts are excluded until the user explicitly pins the source or saves a source search.

### Feed card types

#### Media card

Shows:

- Blurred or visible thumbnail
- Title
- Media type
- Source
- Added or published time
- Duration or page count
- Progress
- Quick actions

#### Chapter update

Shows:

- Series
- Chapter
- Source
- Previous reading progress
- Read action

#### Source notice

Uses:

- Moonstone for information
- Yellow for degraded or rate-limited
- Red for blocked or revoked

#### Download completion

Uses aquamarine during progress and mint when complete.

### Home sections

1. Continue
2. Personalized Feed
3. Recently Added
4. Pinned Collections
5. Source Status

### Feed controls

```text
All | Local | Sources | Chapters | Downloads | Notices
```

Additional controls:

- Mark all seen
- Mute source
- Hide this card type
- Feed settings
- Refresh
- Compact or comfortable density

## Library screen

### Purpose

Browse, filter, select, organize, and open the normalized library.

### Toolbar

```text
[All media] [Search library…] [Filters] [Sort] [Grid/List] [Select]
```

### Left filter panel

- Media type
- Source
- Local or remote
- Creator
- Series
- Tags
- Language
- Viewed state
- Favourite
- Download state
- Duration or pages
- Date added

### Grid

Reference card:

```text
220 × 312 px
```

Card states:

- Default: neutral border
- Hover: raised surface
- Selected: iris border and lavender glow
- Downloaded: mint local badge
- Active playback: seafoam progress
- Warning: yellow status dot and label
- Blocked: no thumbnail, red label, restricted actions

### Detail panel

Shows:

- Large preview
- Resume, Play, Read, or View
- Favourite
- Add to collection
- Queue
- Source attribution
- Progress
- Tags
- Description
- Variants
- Technical information

## Viewer screen

### Video viewer

Layout:

- Edge-to-edge black media stage
- Auto-hiding control bar
- Queue drawer
- Metadata drawer
- Chapter or subtitle controls
- Source and variant indicator

Playback colour:

- Seafoam: play state
- Aquamarine: played progress
- Moonstone: buffered progress
- Yellow: reconnecting
- Red: unrecoverable error

### Image viewer

- Black stage
- Zoom and pan
- Filmstrip
- Metadata drawer
- Slideshow controls
- Source link
- Optional neutral metadata mode

### Story viewer

- Center reading column
- Lavender selection and search highlights
- Indigo chapter navigation
- Yellow bookmark marker
- Mint completed-chapter state
- Optional serif reader font

### Comic viewer

- Page stage
- Reading-direction control
- Chapter drawer
- Page scrubber
- Yellow page marker
- Aquamarine preload indicator
- Red corrupt-page error

## Mock interaction notes

- Opening an item keeps the Library query and scroll position.
- Esc closes drawers before leaving the viewer.
- Space controls playback only when text input is not focused.
- Home feed cards can be dismissed without altering the source item.
- Blocking from Home removes the item before the next paint.
- A red destructive action always requires confirmation.
