# Expanded Source and Site Catalogue

## Integration tiers

Veloura must distinguish between four source types.

### API-backed connectors

These use an API, OPDS catalogue, feed, or protocol intended for programmatic access.

### Personal-server connectors

These connect to a server controlled by the user.

### Metadata-only connectors

These enrich local items but do not provide playback or browsing media.

### Browser handoff templates

These open a named site in the system browser. They do not scrape pages, import cookies, resolve hidden media URLs, or offer downloads.

The implementation-ready preset list will be stored in:

- `assets/source-presets.json` (planned — not yet created; generated from the catalogue below)

## Bundled local sources

- Local folders
- Managed library
- Watch folder
- Removable drive
- Generic WebDAV
- S3-compatible storage
- Generic OPDS 1/2
- RSS
- Atom
- JSON Feed
- M3U or authorized playlist
- EPUB, CBZ, CBR, ZIP and local HTML import

## Personal media servers

### Recommended first-class presets

- Stash
- Hydrus Network
- Jellyfin
- Kavita
- Komga
- Audiobookshelf

### Additional candidate presets

- Emby
- Plex
- Calibre-Web
- Ubooquity
- LANraragi
- Suwayomi-compatible server

Every personal server is disabled until the user provides an address and credentials. Remote connections should require HTTPS; plain HTTP is limited to explicit local-network configuration.

## Booru and image-board presets

### Named presets

- Danbooru
- Gelbooru
- e621
- e926
- Safebooru
- Rule34.xxx, subject to API and policy review

### Generic presets

- Danbooru-compatible
- Gelbooru-compatible
- Moebooru-compatible
- Shimmie-compatible
- Custom JSON booru schema

Initial capability should be read-only:

- Search
- Browse
- Tags
- Pools or galleries
- Item details
- Authorized viewing

Do not include uploading, voting, comments, remote favourites, or moderation actions in the initial connector.

## Metadata-only presets

- StashDB
- ThePornDB
- FansDB
- JAVStash
- PMV Stash

Metadata connectors may supply:

- Title
- Date
- Creator or performer records
- Studio
- Tags
- External IDs
- Fingerprint matches
- Scene or gallery relationships

The user reviews matches before local metadata is changed.

## Browser handoff candidates

Candidate templates include:

### Video and creator platforms

- Pornhub
- XVideos
- XNXX
- xHamster
- YouPorn
- RedTube
- SpankBang
- ManyVids
- Fansly
- OnlyFans
- Clips4Sale

### Live platforms

- Chaturbate
- Stripchat

### Stories, illustration, manga and community

- Literotica
- Hentai Foundry
- E-Hentai
- nhentai
- Hitomi
- F95zone

These are not equivalent to API connectors. Each candidate requires source-policy, age-assurance, regional, security, and trademark review before being included in a public build.

## Source card controls

```text
Enabled
Include in unified search
Show on Home
Show recent feed
Show followed-only feed
Show thumbnails
Blur thumbnails
Cache metadata
Cache thumbnails
Allow downloads
Use in private session
Retain searches
Allowed ratings
Orientation categories
Blocked categories
Blocked tags
Open externally
```

## Source readiness states

This is the catalogue's integration classification for a connector — how it was built, vetted, and maintained. It is independent of a configured source's live connection status (see [29 — Brand, Source Presets, and Aesthetic](29-brand-source-presets.md)'s "Source status labels", e.g. Ready, Disabled, Rate limited): a `Bundled` connector can be `Disabled`, and a `Community` connector can be `Ready`.

- Bundled
- Recommended
- Beta
- Community
- User-defined
- Policy review
- API review
- Degraded
- Revoked
- Browser handoff only

## Evidence and maintenance notes

The implementation plan is based on current official documentation for Stash GraphQL, Hydrus Client API, Jellyfin, Kavita, Komga, Danbooru-family APIs, Gelbooru DAPI, e621-style APIs, and Stash-box metadata instances. Connector capabilities must still be verified during implementation and again before each release.
