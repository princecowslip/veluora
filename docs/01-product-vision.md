# Product Vision

## Summary

Veloura is a private, local-first application that unifies legally accessible adult media across local files and user-enabled sources. It provides consistent browsing, search, organization, playback, and reading across desktop, terminal, and scripted workflows.

The application should feel like a personal media library rather than a public social network. It prioritizes user control, minimal data collection, source transparency, resilient connectors, and clear safety boundaries.

## Product statement

For adults who maintain media across multiple lawful sources and file formats, Veloura provides one private interface for discovery, playback, reading, organization, and offline access. Unlike browser bookmark collections or source-specific clients, it normalizes metadata and interaction patterns without hiding where content came from or bypassing source restrictions.

## Goals

### Unified media experience

A user should be able to move between video, images, stories, audio, and sequential art without learning a different navigation system for each source.

### Local ownership of metadata

Favorites, collections, ratings, progress, notes, private tags, blocked terms, and history belong to the user. They should be stored locally by default and remain usable even when a source disappears.

### Source transparency

Every item must clearly show:

- Original source
- Canonical source URL
- Source-specific identifier
- Access method
- Whether downloading is permitted
- Whether cached media may expire
- Original and normalized tags

### Privacy by default

The default installation should:

- Require no account.
- Bind any service only to localhost.
- Disable telemetry.
- Keep explicit content out of notifications.
- Store credentials in the operating system credential manager.
- Provide private sessions and complete history deletion.

### Extensible but controlled

External integrations should use a capability-based connector interface. Plugins receive only declared permissions and should be isolated from credentials, the main database, and unrelated domains.

## Non-goals

The first product is not:

- A content-hosting platform
- A social network
- A public upload service
- A piracy tool
- A DRM removal tool
- A universal browser automation engine
- A performer identification service
- A cloud-first recommendation platform
- A replacement for source moderation or consent verification

## Product principles

1. **Local first:** network features extend a useful offline library rather than define it.
2. **Explicit capability:** unsupported actions are unavailable rather than simulated.
3. **Source respect:** connectors honor source terms, authentication, rate limits, and deletion.
4. **Private defaults:** sensitive metadata is not transmitted or exposed without deliberate action.
5. **Format parity:** reading and playback state work across every supported media class.
6. **Graceful degradation:** one source failure does not prevent use of other sources or the local library.
7. **Review before automation:** destructive deletion, tag merging, and duplicate removal remain user-reviewable.
8. **Safety before breadth:** a smaller, audited source set is preferable to unrestricted coverage.

## Success measures

### User outcomes

- Users can find any saved item through metadata, tags, source, creator, collection, or full-text search.
- Progress resumes accurately across application restarts.
- The same saved search produces equivalent results in GUI, TUI, and CLI.
- Users can clear viewing history, caches, thumbnails, and credentials independently.
- Blocked tags, sources, and creators remain blocked in every interface.

### Engineering outcomes

- Connector failures are contained.
- Database migrations preserve user state.
- Credentials never appear in logs.
- Local API exposure is limited to the configured interface.
- Media parsers and archive handlers tolerate malformed input safely.
- New connectors can be added without changing the player, reader, or collection code.

## Open product questions

- Is the GUI native, webview-based, or browser-hosted over a local service?
- Should encrypted metadata be enabled by default or optional?
- Which media formats are guaranteed versus best-effort?
- What governance model controls the official connector registry?

Two related questions are already settled elsewhere and are not open: OS support tiers are defined in [09 — Terminal UI Specification](09-terminal-ui.md#supported-platforms) and apply product-wide, and remote access defaults to off (see [02 — Scope and Requirements](02-scope-and-requirements.md) and [20 — Privacy and Security](20-privacy-and-security.md)).
