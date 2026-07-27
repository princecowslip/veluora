# User-Friendly Workflows

## Design objective

A new user should be able to install Veloura, add a local folder, open media, and create a collection without reading technical documentation.

Advanced capability should remain available without dominating the default interface.

## First-run experience

### Welcome

Message:

> Veloura brings your local library and selected sources into one private media workspace.

Actions:

- Start setup
- Import existing profile
- Learn about privacy

### Privacy profile

Offer three clear profiles:

#### Personal device

- History on
- Thumbnails visible after unlock
- Local cache enabled
- Auto-lock optional

#### Shared device

- Start locked
- Thumbnails blurred
- Neutral notifications
- Short history retention
- Auto-lock enabled

#### Private-first

- History off
- Session-only source cache
- Start locked
- Private sessions emphasized

Each profile remains editable.

### Add local library

The user selects folders.

The setup screen shows:

- Folder path
- Estimated item count
- Supported formats
- Thumbnail-storage estimate
- Read-only indexing explanation
- Scan now or later

### Test playback

The application finds one supported local item and verifies:

- Video or audio output
- Image rendering
- External player configuration
- Subtitle support

### Optional sources

Sources are presented by category:

- Personal servers
- Catalogues and feeds
- Booru APIs
- Browser handoff
- Developer connectors

The user can skip all network sources.

## Friendly defaults

- Local-first Home screen
- Simple mode
- Grid library view
- Search history with limited retention
- Public sources excluded from Home
- Downloads off for remote sources
- Autoplay off
- Animated previews off
- External thumbnails blurred
- Technical metadata collapsed
- Advanced query help available on demand

## Action language

Prefer direct verbs:

- Play
- Read
- View
- Resume
- Save
- Add to collection
- Open source
- Clear history
- Remove download

Avoid internal terms in the main UI:

- Recompute canonical fingerprint
- Refresh capability snapshot
- Emit `ItemMetadataUpdated` event
- Reconcile source health state
- Rebuild FTS index

Those terms may appear in diagnostics. See [12 — System Architecture](12-system-architecture.md) and [13 — Domain and Data Model](13-data-model.md) for where they come from.

## Smart empty states

### Home

> Continue, feed updates, and recent additions will appear here.

Actions:

- Add folder
- Add source
- Open Library

### Library

> Your library is empty.

Actions:

- Add local folder
- Import files
- Connect personal server

### Discover

> No external sources are enabled.

Actions:

- Add source
- Search local library

### Downloads

> Nothing is downloading.

Actions:

- Open Library
- Review storage settings

## Progressive disclosure

### Basic item details

Show:

- Title
- Creator
- Source
- Media type
- Progress
- Tags
- Primary actions

### More details

Reveal:

- File path
- Codec
- Bitrate
- Hash
- Source item ID
- Temporary URL expiry
- Connector diagnostics

## Undo and recovery

Reversible actions should provide Undo:

- Remove from collection
- Clear queue
- Hide item
- Unfavourite
- Mark complete
- Remove local reference
- Dismiss feed card
- Change preferred variant

Destructive file deletion requires confirmation and cannot rely only on Undo.

## Friendly filtering

### Filter chips

Applied filters appear as removable chips.

Example:

```text
Video ×  Local ×  Unviewed ×  Under 20 min ×
```

### Filter presets

- Continue
- Unviewed
- Downloaded
- Recently added
- Short videos
- Long reads
- New chapters
- Local only
- External only

### Search help

When the user types:

```text
type:
```

The interface suggests valid values.

When a query fails, highlight the exact invalid section.

## Friendly source setup

Each source setup page answers:

1. What does this source provide?
2. What access does Veloura need?
3. What will be stored locally?
4. Can media be downloaded?
5. Can the source appear on Home?
6. How can it be removed?

## Friendly errors

Every error includes:

- What happened
- What still works
- What the user can do
- Optional technical details

Example:

> This source is temporarily rate limited. Local items and other sources are still available. Try again later or reduce refresh frequency.

## Friendly safety controls

Blocking should be understandable.

Example flow:

1. Choose Block.
2. Select item, creator, tag, series, or source.
3. Preview affected item count.
4. Choose all sources or current source.
5. Confirm.
6. Offer Undo.

Policy-blocked content does not offer Undo.

## Friendly privacy controls

The Privacy Center uses plain categories:

- What Veloura remembers
- What Veloura stores
- What sources can see
- What appears on this device
- Clear or export data

## Help system

Provide:

- Inline help
- Shortcut overlay
- Searchable documentation
- Command examples
- Source-specific troubleshooting
- “Why is this shown?” explanations for feed and recommendations
- Privacy explanations
