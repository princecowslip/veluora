# UI and UX Principles

## Experience goals

The interface should feel:

- Private
- Calm
- Fast
- Media-centered
- Keyboard-friendly
- Clear about source and permissions
- Consistent across formats
- Non-judgmental without becoming careless about safety

## Core interaction principles

### Reveal sensitive content deliberately

Before unlock, show neutral placeholders. After unlock, remember the state only for the configured session duration.

Optional controls:

- Blur thumbnails globally
- Blur only external source results
- Require hover or focus to reveal
- Disable preview video
- Hide titles until selected

### Preserve context

Opening a player or reader should not discard:

- Current search
- Scroll position
- Selected filters
- Source result state
- Queue
- Sort order

Back navigation returns users to the exact prior location.

### Show progressive results

Unified search must not wait for every source. Render source groups as they complete and clearly label:

- Loading
- Complete
- No results
- Unsupported query
- Authentication required
- Rate limited
- Failed

### Make capability visible

Do not show inactive controls without explanation. For example:

- If downloading is unavailable, show “Open at source” rather than a disabled download button.
- If a source cannot filter by duration, apply the filter locally and explain that results may be incomplete.
- If a media URL has expired, offer refresh rather than generic playback failure.

### Separate local state from source state

Use distinct labels:

- Local favorite
- Source favorite
- Local tags
- Source tags
- Local download
- Source bookmark

Avoid implying that local actions modify the remote source.

## Onboarding flow

### Step 1: Adult and safety notice

- Confirm lawful adult use.
- Summarize prohibited uses.
- Link to privacy and content policies.
- Avoid collecting identity information at this stage.

### Step 2: Privacy setup

Choices:

- Standard local mode
- Encrypted metadata mode
- Shared-device mode

Allow later changes.

### Step 3: Local library

- Select folders.
- Explain read-only indexing.
- Show supported formats.
- Estimate storage needed for thumbnails and metadata.

### Step 4: Playback integration

- Select built-in or external players.
- Test one sample file.
- Configure hardware decoding and subtitle defaults.

### Step 5: Optional sources

- Add only user-selected sources.
- Show permissions and terms.
- Explain that source credentials remain separate.

## Empty states

Each empty state should explain the next meaningful action.

Examples:

- Empty Library: “Add a folder to start indexing.”
- Empty Discover: “Enable a source or search your local library.”
- Empty Downloads: “Downloads appear only when a source permits them.”
- Empty Continue: “Items with saved progress will appear here.”

## Error design

Errors need:

- Plain-language summary
- Scope of failure
- Whether user data is safe
- Recovery action
- Technical details behind a disclosure
- Redacted copy button

Example:

> Playback link expired. Your library entry and progress are safe. Refresh the media link or open the original source.

## Confirmation design

Require confirmation for:

- Clearing all history
- Deleting downloads
- Removing credentials
- Removing a library root
- Applying a broad block rule
- Resetting the database
- Enabling remote access
- Installing a high-permission plugin

Avoid confirmation for reversible actions such as favorite, queue, and collection changes.

## Keyboard model

Global:

```text
Ctrl/Cmd+K   Command palette
/            Focus search
Esc          Close, back, or leave fullscreen
Space        Play or pause
F            Favorite
Q            Add to queue
C            Add to collection
I            Toggle information panel
?            Shortcut help
```

Viewer and reader shortcuts adapt by media type but must not conflict with global privacy actions.

## Privacy Center

The Privacy Center should be a first-class destination, not a buried settings subsection.

It should show:

- Current lock status
- History retention
- Thumbnail behavior
- Cache size
- Download size
- Stored source credentials
- Telemetry status
- Last data-clearing operation
- Export and deletion controls
