# Features and Functions

## Implementation status

This document is a design target covering the complete intended feature
set, present-tense throughout, with no shipped/planned distinction. The
shipped feature set is much smaller — see `CHANGELOG.md` for what exists
milestone-by-milestone and `KNOWN_ISSUES.md` for explicit gaps. Notably
unbuilt: the command palette, the simple/advanced mode toggle, saved
searches, duplicate-review UI, most GUI collection functions (the GUI has
no top-level Collections screen at all), and most Viewer story features
(text-to-speech, highlighting, notes). Treat every section below as
intent, not current behavior, unless cross-checked against the
CHANGELOG.

## Purpose

This document defines the complete user-facing and system-facing feature set for Veloura.

The product should feel simple at first use while still supporting advanced filtering, automation, external players, scripts, and source connectors.

## Product modes

### Simple mode

Default for new users.

Shows:

- Home
- Library
- Discover
- Collections
- Downloads
- Settings
- Essential filters
- Primary actions
- Guided source setup

Hides:

- Raw connector capabilities
- Advanced query syntax
- Plugin permissions beyond summaries
- Technical metadata
- Developer diagnostics
- Complex automation

### Advanced mode

Optional.

Adds:

- Full query language
- Connector diagnostics
- Plugin controls
- Technical media variants
- Batch metadata editing
- External command templates
- Local API access
- Script-friendly identifiers
- Advanced cache and retention settings

## Core navigation functions

### Global search

Searches:

- Local library
- Enabled personal servers
- Enabled public sources
- Collections
- Creators
- Series
- Tags
- Notes, when local search includes private data
- Downloads
- Settings and commands

Search suggestions should identify scope:

```text
Local item
Source result
Collection
Creator
Command
Setting
```

### Command palette

The command palette provides keyboard access to:

- Open Home
- Search current view
- Create collection
- Start private session
- Lock application
- Clear current filters
- Resume last item
- Open downloads
- Add source
- Run diagnostics
- Change theme
- Toggle simple or advanced mode

### Back and history

The application keeps an internal navigation stack.

Functions:

- Back
- Forward
- Reopen last closed item
- Restore prior filter and scroll position
- Open item in new window
- Pin current view

## Home functions

### Continue

Shows partially viewed or read items.

Actions:

- Resume
- Restart
- Mark complete
- Remove from Continue
- Open details

### Personalized feed

Feed sources:

- Local additions
- New items from personal servers
- Followed series updates
- Saved-search matches
- Queue activity
- Download completion
- Source warnings
- Plugin or connector updates

Feed functions:

- Filter by type
- Filter by source
- Mark seen
- Mark all seen
- Mute source
- Hide card type
- Snooze notices
- Pin item
- Save to collection
- Open source
- Block item or tag

### Quick actions

Home may show:

- Add local folder
- Add source
- Resume last item
- Open queue
- Start private session
- Clear session history
- Scan library
- View storage status

## Library functions

### Browse

View modes:

- Poster grid
- Compact grid
- Detailed list
- Table
- Timeline
- Creator shelf
- Series shelf
- Chapter list

### Filter

Filter groups:

- Media type
- Source
- Local or remote
- Creator
- Series
- Tag
- Language
- Duration
- Page count
- Resolution
- File format
- Date added
- Publication date
- Viewed state
- Completion state
- Favourite
- Rating
- Downloaded
- Collection
- Safety or visibility status

### Sort

Sort options:

- Recently added
- Recently opened
- Publication date
- Title
- Creator
- Duration
- Page count
- Rating
- Progress
- Random
- Source order
- Manual collection order

### Batch actions

For selected items:

- Add to collection
- Remove from collection
- Add or remove private tags
- Mark viewed or unviewed
- Mark complete
- Favourite or unfavourite
- Queue
- Download when allowed
- Export metadata
- Re-index
- Hide
- Block
- Remove local reference
- Delete local files with confirmation

### Duplicate review

Functions:

- Compare metadata
- Compare quality
- Compare source
- Select preferred variant
- Merge logical items
- Keep separate
- Hide duplicate variants
- Delete duplicate local files after review

## Discover functions

### Unified search

Searches enabled sources progressively.

Functions:

- Select source scope
- Save search
- Pin search to Home
- Group by source
- Merge duplicates
- Sort locally
- Open original source
- Save reference
- Add to queue
- Download when explicitly permitted
- Block item, creator, source, or tag

### Source browse

Source-specific browse routes may include:

- Recent
- Popular
- Tags
- Creators
- Series
- Categories
- Pools
- Playlists
- Chapters

The UI clearly labels source-provided ranking.

## Item detail functions

### Primary actions

Depending on media type:

- Play
- Resume
- Read
- View
- Open gallery
- Open original source

### Organization

- Favourite
- Rate
- Add to collection
- Queue
- Add private tags
- Add note
- Mark viewed
- Mark complete
- Pin

### Source and variant actions

- Select quality
- Select language
- Select subtitle
- Select local or remote variant
- Refresh expired link
- Open source
- Copy canonical URL
- View source metadata
- View local overrides

### Safety and privacy actions

- Blur item
- Hide item
- Block item
- Block creator
- Block tag
- Block source
- Exclude from history
- Remove from recent activity

## Viewer functions

### Video

- Play and pause
- Seek
- Jump forward or backward
- Volume
- Mute
- Playback speed
- Quality
- Subtitle selection
- Audio track selection
- Fullscreen
- Picture-in-picture where supported
- Chapter navigation
- Queue navigation
- Loop
- Screenshot only for local media or when source permission allows
- External player
- Resume
- Mark complete

### Image and animation

- Zoom
- Pan
- Rotate
- Mirror
- Fit width
- Fit height
- Original size
- Slideshow
- Animation pause
- Frame stepping where supported
- Filmstrip
- Metadata overlay
- External viewer

### Story

- Font size
- Font family
- Line height
- Line width
- Theme
- Chapter navigation
- Search in story
- Bookmark
- Notes
- Highlight
- Text-to-speech through local or approved provider
- Reading progress
- Distraction-free mode

### Manga and comics

- Left-to-right
- Right-to-left
- Single page
- Double page
- Vertical strip
- Fit width
- Fit height
- Margin crop
- Page preloading
- Page scrubber
- Chapter list
- Bookmark
- Resume
- External reader

### Audio

- Play and pause
- Seek
- Speed
- Volume
- Sleep timer
- Chapter navigation
- Queue
- Background playback
- Resume
- External player

## Collection functions

### Manual collection

- Create
- Rename
- Describe
- Add cover
- Add items
- Reorder items
- Sort
- Export
- Duplicate
- Share metadata export without media
- Delete with undo

### Smart collection

- Build from query
- Preview results
- Refresh automatically
- Pin to Home
- Convert to manual
- Export query
- Disable external sources

## Queue functions

- Add next
- Add to end
- Reorder
- Remove
- Clear
- Save queue as collection
- Shuffle
- Repeat item
- Repeat queue
- Continue across media types
- Choose external player per item type

## Download functions

- Queue download
- Select quality
- Select destination
- Pause
- Resume
- Retry
- Cancel
- Verify
- Reveal file
- Move file
- Remove downloaded copy
- Preserve library reference
- Apply storage rule
- Mark as permanent or cache

## Source functions

- Enable
- Disable
- Configure
- Authenticate
- Test
- Refresh
- Inspect permissions
- Set thumbnail behavior
- Set history retention
- Set cache policy
- Set download permission
- Pin source feed
- Mute feed
- Set rating mapping
- Set block rules
- Remove credentials
- Remove source

## Privacy functions

- Lock
- Auto-lock
- Private session
- Neutral mode
- Hide titles
- Blur thumbnails
- Disable preview
- Clear search history
- Clear viewing history
- Clear thumbnails
- Clear cache
- Remove credentials
- Export private data
- Delete profile
- Verify deletion
- Redact notifications
- Prevent operating-system recent-window previews where supported

## Accessibility functions

- Keyboard-only operation
- Screen-reader labels
- High contrast
- Reduced motion
- Interface scaling
- Reader typography
- Monochrome TUI
- Plain CLI output
- Caption and subtitle controls
- Custom keyboard shortcuts
