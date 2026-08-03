# Final Product Summary

## Product

Veloura is a private, local-first media browser, library, aggregator, viewer, reader, and player for lawful adult content.

It supports:

- Video
- Images
- Animated images
- Stories
- Audio
- Manga
- Comics
- Galleries
- Chapters
- Playlists
- Local files
- Personal servers
- Approved API connectors
- Browser handoff sources

## Interfaces

- Desktop GUI
- Terminal UI built with C++20 and notcurses
- CLI
- Local API for trusted clients

## Signature experience

- Pure-black canvas
- Indigo primary actions
- Lavender and iris selection
- Violet emphasis
- Moonstone information
- Mint success
- Seafoam playback
- Aquamarine progress
- Yellow warnings
- Red destructive and blocked states

## Main navigation

- Home
- Library
- Privacy Center
- Sources
- Discover
- Downloads
- Settings

Collections has no dedicated top-level screen in the GUI today — it's
reachable via `application::CollectionService`/the CLI/the TUI's
Collections view (F3), but not from the desktop GUI's main navigation.

## Home

Home contains:

- Continue
- Personalized feed
- Recently added
- Pinned collections
- Source status
- Storage and indexing activity

The feed is local-first and user-controlled.

## Product promise

> A private gallery for every format.

## MVP

The MVP includes:

- Local library
- GUI and CLI
- Playback and readers
- Search
- Progress
- Favourites
- Collections
- Queue
- Privacy Center
- Source block rules
- Design tokens
- Accessible keyboard operation

## Shipped since MVP

- Unified source search (Discover, Milestone I)
- Download manager (Milestone J)
- Terminal UI (Milestone G)
- Source connectors: RSS/Atom feed, booru (Danbooru/Gelbooru), OPDS
  (Milestones F, K, L)
- Plugin sandbox infrastructure (Milestone H)

## Next release

- Personal server connectors (Jellyfin, Kavita, Komga, and similar — see
  `KNOWN_ISSUES.md`)
- Duplicate review
- Saved searches
- Home feed personalization

## Later

- Sandboxed third-party plugins
- Signed connector registry
- Optional remote mode
- Local-only explainable recommendations
- Advanced automation
