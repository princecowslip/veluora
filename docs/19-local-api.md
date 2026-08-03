# Local API

## Purpose

The local API allows GUI, TUI, CLI, and optional trusted extensions to use one running application core.

## Default security posture

- Bind to loopback only.
- Generate a random authentication token.
- Store the token in the credential manager or protected runtime file.
- Reject browser cross-origin requests by default.
- Disable remote access.
- Rotate tokens when requested.
- Do not include credentials or signed media URLs in logs.

## API style

A versioned HTTP or IPC API can be used. Local operating-system IPC is preferable where cross-platform support is manageable.

Example base:

```text
http://127.0.0.1:{random-port}/api/v1
```

## Core endpoints

### Health

```text
GET /health
GET /diagnostics/summary
```

### Library

```text
GET    /library/roots
POST   /library/roots
DELETE /library/roots/{id}
POST   /library/scan
GET    /library/status
```

### Search

```text
POST /search
GET  /search/{search_id}/events
DELETE /search/{search_id}
```

Streaming results may use server-sent events, WebSocket, or IPC events.

### Items

Shipped routes (`crates/local-api/src/routes/items.rs`):

```text
GET   /items/{id}
POST  /items/{id}/favorite
POST  /items/{id}/pin
POST  /items/{id}/open
POST  /items/{id}/progress
GET   /items/{id}/story
GET   /items/{id}/pages
```

There is no `PATCH /items/{id}/metadata` route yet.

### Discover

```text
POST /discover
```

Aggregates the local library with every enabled, non-local connector
source in one call (`application::DiscoverService`, Milestone I).

### Home

```text
GET /home/continue
```

### Collections

Shipped routes (`crates/local-api/src/routes/collections.rs`):

```text
GET    /collections
POST   /collections
DELETE /collections/{id}
POST   /collections/{id}/items
DELETE /collections/{id}/items/{item_id}
```

There is no single-resource `GET /collections/{id}` or `PATCH
/collections/{id}` route yet.

### Sources

Shipped routes (`crates/local-api/src/routes/sources.rs`):

```text
GET    /sources
POST   /sources
DELETE /sources/{id}
POST   /sources/{id}/enable
POST   /sources/{id}/disable
POST   /sources/{id}/health-check
POST   /sources/{id}/browse
POST   /sources/{id}/import
```

There is no single-resource `PATCH /sources/{id}`, `POST
/sources/{id}/authenticate`, or `DELETE /sources/{id}/credentials` route —
connector configuration (including any credentials) is passed as opaque
`configuration_json` on create; there is no separate authenticate step or
credential-manager integration yet.

### Downloads

Shipped routes (`crates/local-api/src/routes/downloads.rs`, Milestone J):

```text
GET    /downloads
POST   /downloads
GET    /downloads/status
GET    /downloads/quota
POST   /downloads/enforce-quota
POST   /downloads/eligibility
POST   /downloads/{id}/pause
POST   /downloads/{id}/resume
POST   /downloads/{id}/cancel
POST   /downloads/{id}/pin
DELETE /downloads/{id}
```

`add`/`resume` spawn the fetch on `local-api`'s own runtime and return
`202` immediately, since it's the only long-lived surface that can run a
download to completion in the background.

### Privacy

Shipped routes (`crates/local-api/src/routes/privacy.rs`):

```text
GET  /privacy/status
POST /privacy/verify
```

There is no `POST /privacy/lock`, `POST /privacy/private-session`, or
`POST /privacy/clear` route — those actions aren't exposed over the local
API today.

### Blocks

Not implemented. `domain::BlockRule` exists and is consulted inside
`DownloadService`'s eligibility check (Milestone J), but there is no
`local-api` route, CLI command, or GUI/TUI surface to create, list, or
remove a block rule — no CRUD API for blocking exists at all yet. This gap
is not currently tracked in `KNOWN_ISSUES.md` either; see that file's
Connectors/Privacy sections for the nearest related entries.

## Authorization scopes

Potential local client scopes:

- read_library
- modify_library
- playback
- manage_downloads
- manage_sources
- manage_plugins
- privacy_admin
- diagnostics

The GUI may have all scopes. Extensions should receive narrower tokens.

A plugin's "Local API" capability (see [18 — Plugin System](18-plugin-system.md)) is granted as an explicit subset of these scopes, declared in the plugin manifest and enforced the same way as any other client token — plugins never receive a separate or broader permission surface.

## Event stream

Events may include:

- indexing progress
- thumbnail ready
- search source completed
- playback progress
- download state changed
- source health changed
- privacy lock changed

Event payloads should omit sensitive text when the client is locked.

## Remote access

Remote access is a separate feature with:

- TLS
- user authentication
- device authorization
- revocation
- rate limiting
- audit events
- explicit network binding
- clear warning

It should not be enabled by changing only a host address.
