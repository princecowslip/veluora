# Information Architecture

## Navigation model

The desktop product uses six stable top-level destinations:

1. Home
2. Library
3. Discover
4. Collections
5. Downloads
6. Settings

A compact player or reader status area remains available across the application.

## Home

Purpose: resume activity and expose high-value shortcuts without becoming an explicit-content dashboard when the application is locked.

Sections:

- Continue
- Queue
- Personalized feed
- Recently added
- Pinned collections
- Saved searches
- Source status
- Storage and indexing activity
- Optional local recommendations

The Personalized Feed combines local activity, pinned-source updates, followed-series chapters, saved-search matches, download completions, and source notices. Public-source feed items remain off until the user explicitly pins that source or search.

Locked mode replaces sensitive titles and thumbnails with neutral placeholders.

## Library

Purpose: browse indexed local and saved media.

Primary views:

- All items
- Videos
- Images and animated images
- Stories
- Audio
- Manga and comics
- Galleries and series
- Creators
- Tags
- Duplicates
- Unmatched metadata

Library filters are local and never sent to external sources.

## Discover

Purpose: search and browse enabled external sources.

Subsections:

- Unified search
- Per-source browse
- Trending or recent, only where explicitly supplied by the source
- Saved source searches
- Source status and authentication warnings

The interface must distinguish source-provided ranking from local ranking.

## Collections

Types:

- Manual collection
- Smart collection
- Queue
- Favorites
- Recently viewed
- Completed
- Hidden or blocked review area

Collections may contain references, local files, or both.

## Downloads

Sections:

- Active
- Queued
- Completed
- Failed
- Cached
- Storage rules

Each entry shows whether it is a temporary cache object or a permanent user download.

## Settings

Groups:

- General
- Appearance
- Playback and reading
- Library locations
- Sources
- Plugins
- Privacy
- Safety and blocked content
- Storage
- Keyboard shortcuts
- Diagnostics
- About and licenses

## Item detail model

Every media item detail page follows the same hierarchy:

1. Preview or poster
2. Title and media type
3. Primary actions
4. Source and access status
5. Progress
6. Creator, series, tags, language, and technical metadata
7. Description
8. Variants, chapters, or gallery pages
9. Related items
10. Local user state and notes

## Action hierarchy

### Primary actions

- Play
- Read
- View
- Resume
- Open original source

### Secondary actions

- Favorite
- Add to collection
- Queue
- Download when permitted
- Open externally

### Overflow actions

- Edit local metadata
- Copy canonical URL
- Block item
- Block source, creator, or tag
- Re-index
- View technical information
- Report connector issue

## URL and deep-link model

Internal routes should be stable enough for bookmarks and automation:

```text
veloura://home
veloura://library
veloura://item/{uuid}
veloura://collection/{uuid}
veloura://source/{source_id}
veloura://search?q={encoded_query}
veloura://downloads
veloura://settings/privacy
```

Deep links must not contain credentials or temporary signed media URLs.
