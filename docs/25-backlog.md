# Backlog and Acceptance Criteria

## Epic: Local indexing

### Add library root

Acceptance criteria:

- User can select a readable directory.
- Application explains that indexing is read-only by default.
- Duplicate roots are rejected.
- Unsupported paths show a clear error.
- Root appears in library settings.
- Indexing can start immediately or later.

### Scan library

Acceptance criteria:

- Supported files create items or variants.
- Unsupported files are skipped and counted.
- Scan can pause and resume.
- Existing user state is preserved.
- Progress is visible.
- Cancellation leaves database consistent.

## Epic: Search

### Structured query parser

Acceptance criteria:

- Supports include and exclude fields.
- Returns position-aware syntax errors.
- Preserves quoted phrases.
- Produces a serializable AST.
- Rejects unknown fields with suggestions.

### Progressive unified search

Acceptance criteria:

- Local results can appear before remote results.
- Each source has visible status.
- One failure does not cancel other sources.
- User can cancel the complete search.
- Result groups preserve source attribution.

## Epic: Playback

### Resume video

Acceptance criteria:

- Progress saves periodically.
- Progress saves on pause, seek, close, and crash recovery where possible.
- Resume prompt appears when meaningful progress exists.
- Completion threshold is configurable.
- Restart-from-beginning clears old progress after confirmation.

## Epic: Reader

### Manga direction

Acceptance criteria:

- User can select left-to-right or right-to-left.
- Choice may be stored per series.
- Page keyboard navigation follows direction.
- Double-page ordering is correct.
- Current page survives restart.

## Epic: Privacy

### Clear viewing history

Acceptance criteria:

- User can preview affected data categories.
- Clearing does not delete media.
- Continue and Recently Viewed update immediately.
- FTS and derived history records are removed.
- Operation produces a privacy-safe audit result.

### Lock application

Acceptance criteria:

- Content and titles hide immediately.
- Playback pauses according to settings.
- Thumbnail windows and overlays are obscured.
- Unlock uses configured authentication.
- Failed unlock attempts are rate limited.

## Epic: Sources

### Add source

Acceptance criteria:

- Connector permissions are shown.
- Unsupported or revoked connector cannot be enabled.
- Authentication is tested.
- Capabilities are displayed.
- Source may be disabled without deleting local references.

## Epic: Download

### Authorized download

Acceptance criteria:

- Action appears only when capability and variant permission allow.
- Destination and estimated size are shown.
- Partial file uses a temporary extension.
- Completion verifies size or checksum.
- Source attribution is retained.

## Epic: Blocking

### Block tag

Acceptance criteria:

- User sees rule scope.
- Existing affected count is estimated.
- Block applies before thumbnails render.
- Rule applies to GUI, TUI, CLI, and API.
- Rule can be disabled or removed.
