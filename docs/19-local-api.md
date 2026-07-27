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

```text
GET   /items/{id}
PATCH /items/{id}/metadata
POST  /items/{id}/open
POST  /items/{id}/progress
POST  /items/{id}/favorite
```

### Collections

```text
GET    /collections
POST   /collections
GET    /collections/{id}
PATCH  /collections/{id}
DELETE /collections/{id}
POST   /collections/{id}/items
DELETE /collections/{id}/items/{item_id}
```

### Sources

```text
GET    /sources
POST   /sources
GET    /sources/{id}
PATCH  /sources/{id}
POST   /sources/{id}/health-check
POST   /sources/{id}/authenticate
DELETE /sources/{id}/credentials
```

Authentication should launch a secure interactive flow rather than accept raw passwords in ordinary JSON where possible.

### Downloads

```text
GET    /downloads
POST   /downloads
POST   /downloads/{id}/pause
POST   /downloads/{id}/resume
DELETE /downloads/{id}
```

### Privacy

```text
GET  /privacy/status
POST /privacy/lock
POST /privacy/private-session
POST /privacy/clear
```

### Blocks

```text
GET    /blocks
POST   /blocks
DELETE /blocks/{id}
```

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
