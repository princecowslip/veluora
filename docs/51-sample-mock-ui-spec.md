# Sample Mock UI Specification

## Purpose

This document provides sample screen-level UI specifications for **Home**, **Library**, and **Viewer**. It is not a final wireframe, but a concrete reference for layout, components, states, and interactions. See also [32 — UI/UX Wireframes](32-ui-wireframes.md) for structural ASCII wireframes and [52 — Sample UI Specification](52-sample-ui-spec.md) for exact pixel measurements of the same screens — the three documents are complementary, not contradictory.

This is a target-state depiction, not current behavior: the real Home
screen has only Continue and Recently Added sections (no Queue, Pinned
Collections, or Saved Searches), and the real GUI has no top-level
Collections screen.

## 1. Home Screen

### Goals

- Resume activity quickly
- Show useful entry points without turning the home screen into a noisy explicit-content feed
- Keep privacy state visible at all times

### Desktop layout

```text
┌────────────────┬─────────────────────────────────────────────────────────┐
│ Left Nav       │ Top Bar                                                 │
│                │ [Search…] [Cmd] [Private] [Lock] [Profile]             │
│ Home           ├─────────────────────────────────────────────────────────┤
│ Library        │ Continue                                                │
│ Discover       │ [card] [card] [card] [card]                            │
│ Collections    │                                                         │
│ Downloads      │ Queue                                                   │
│ Settings       │ [row items with progress bars]                          │
│                │                                                         │
│ Player Status  │ Pinned Collections                                      │
│                │ [collection cards]                                      │
│                │                                                         │
│                │ Saved Searches                  Source Status            │
│                │ [chips/buttons]                 [status list]            │
└────────────────┴─────────────────────────────────────────────────────────┘
```

### Sections

#### Top Bar
- Search field
- Command palette button
- Private-session toggle
- Lock button
- Profile or settings shortcut

#### Continue
- Horizontal card rail
- Card shows thumbnail, title, source badge, progress bar, media type, last-opened time
- Max 8 visible cards with “See all”

#### Queue
- Compact list with title, type, duration/pages, source, and remove action

#### Pinned Collections
- 2xN card grid
- Cover montage or representative cover art
- Collection count

#### Saved Searches
- Chip row
- Clicking a chip opens Library or Discover with that query applied

#### Source Status
- Ready
- Disabled
- Setup required
- Authentication required
- Rate limited
- Offline

### Home interactions

- Clicking a Continue card resumes directly
- Hover on cards reveals quick actions: Favorite, Add to Collection, Open Details
- In private mode, titles can be masked and thumbnails blurred
- External discovery feeds are hidden unless explicitly pinned by the user

## 2. Library Screen

### Goals

- Support dense browsing
- Provide fast filtering and sorting
- Preserve context while moving between list, grid, and detail views

### Wide-screen layout

```text
┌────────────────┬──────────────────────────────────────┬──────────────────┐
│ Filter Panel   │ Library Toolbar                      │ Detail Pane      │
│                │ [Type tabs] [Search] [Sort] [View]  │                  │
│ Sources        ├──────────────────────────────────────┤ Preview          │
│ Tags           │ Result Grid/List                     │ Title            │
│ Creators       │ [media card] [media card]            │ Actions          │
│ Series         │ [media card] [media card]            │ Metadata         │
│ Ratings        │ [media card] [media card]            │ Tags             │
│ Status         │ [media card] [media card]            │ Variants         │
│                │                                      │ Notes            │
└────────────────┴──────────────────────────────────────┴──────────────────┘
```

### Toolbar

- Media type tabs: All, Video, Images, Stories, Audio, Manga/Comics
- Search field
- Filter button
- Sort selector
- View selector: Grid / Compact List / Detailed List
- Selection mode
- “New Collection” button

### Media cards

Show:

- Thumbnail
- Media type badge
- Duration or page count
- Progress indicator
- Source badge
- Favorite marker
- Download/cached state

### Filter categories

- Source
- Type
- Creator
- Series
- Tag
- Language
- Duration / Pages
- Viewed / Unviewed
- Favorite
- Downloaded
- Blocked hidden toggle

### List modes

#### Grid
Best for visual browsing.

#### Compact list
Best for density and keyboard navigation.

#### Detailed list
Best for metadata-rich comparison across items.

### Detail pane

Displays:

- Larger preview
- Title
- Primary actions: Play / Read / View / Resume / Open Source
- Secondary actions: Favorite / Queue / Collection / Download
- Description
- Tags
- Technical metadata
- Local notes
- Variants / chapters / gallery pages

### States

- Empty: “Add a folder or enable a source to start building your library.”
- No results: “Try removing a filter or saving a broader search.”
- Loading: skeleton grid or rows
- Blocked results: visible count, hidden items not shown by default

## 3. Viewer Screen

The Viewer is a shared shell for image, video, story, audio, and comic experiences.

### Shared viewer frame

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Back  Title                     Source      Favorite Queue Info More     │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                         Main media surface                               │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Timeline / page scrubber / chapter bar                                  │
│ Controls row or reading toolbar                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## 3A. Video Viewer

### Main controls
- Play/pause
- Seek
- Time elapsed / remaining
- Volume
- Speed
- Subtitle menu
- Audio track menu
- Quality selector
- Fullscreen
- Picture-in-picture where supported

### Optional side panel
- Queue
- Chapters
- Metadata
- Related items

### States
- Buffering
- Expired link
- Authentication required
- Playback error
- Download not permitted

## 3B. Image Viewer

### Features
- Zoom
- Pan
- Fit width / height / original size
- Rotate
- Slideshow
- Previous / next in gallery
- Filmstrip strip on demand

### Layout note
The image surface should float over the pure-black canvas with minimal chrome.

## 3C. Story Reader

### Reader layout
- Center column
- Adjustable width
- Typography toolbar
- Chapter list
- Search in story
- Bookmark
- Progress indicator

### Reading options
- Sans, serif, and dyslexia-friendly options
- Line height
- Font size
- Theme: dark, light, extra dim

## 3D. Manga / Comic Viewer

### Features
- Single page
- Double page
- Right-to-left
- Left-to-right
- Vertical strip
- Fit width / height
- Margin crop
- Chapter selector

### Page controls
- Page scrubber
- Page number
- Quick jump
- Toggle info

## 4. Color application examples

- Primary nav active item: **Indigo**
- Quick hover glow: **Lavender**
- Active toggle / selected commands: **Iris**
- High-emphasis selected state: **Violet**
- Source badge: **Moonstone**
- Completed download / success: **Mint**
- Playback / active progress: **Seafoam**
- Discovery highlight: **Aquamarine**
- Warning banners: **Yellow**
- Error / blocked state: **Red**

## 5. Interaction principles in mock screens

- Keep chrome minimal when media is active
- Preserve the user’s place when closing detail or viewer
- Never autoplay previews on Home by default
- Blur or mask thumbnails in locked/shared-device mode
- Do not mix too many accent colors on one surface
- Use color for hierarchy, not decoration
