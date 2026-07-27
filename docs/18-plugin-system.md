# Plugin System

## Plugin types

- Source connector
- Metadata provider
- Thumbnail provider
- External player adapter
- Importer
- Exporter

The first third-party API should support source connectors only. Expand after the permission model is proven.

## Security model

Plugins run outside the main process.

Default-deny capabilities:

- Network
- Filesystem
- Credentials
- Process execution
- Clipboard
- Local API
- Notifications

A plugin's granted "Local API" capability maps onto a specific, declared subset of the authorization scopes defined in [19 — Local API](19-local-api.md) (for example `read_library` or `playback`), never onto blanket API access. The plugin manifest's `permissions` block should list the exact scopes requested.

## Manifest

```yaml
id: org.example.connector
name: Example Source
version: 1.2.0
publisher: Example
api_version: 1
entrypoint: plugin.wasm
permissions:
  network:
    domains:
      - api.example.test
  credentials:
    scopes:
      - source
  filesystem:
    read: []
    write: []
capabilities:
  - search
  - browse
  - item_details
media_types:
  - image
```

## Runtime options

### WebAssembly sandbox

Advantages:

- Strong capability model
- Portable
- Easier resource limits

Challenges:

- Ecosystem constraints
- Media and networking APIs need host bindings

### Separate native process

Advantages:

- Language flexibility
- Easier migration for existing connectors

Challenges:

- Harder sandboxing across operating systems
- Larger attack surface

A hybrid approach may support first-party native connectors and third-party WebAssembly connectors.

## Permissions UI

Before installation, show:

- Publisher
- Signature status
- Requested domains
- Credential scope
- Filesystem access
- External process access
- Data categories returned
- Update policy

Permission changes during an update require fresh approval.

## Package signing

Official registry packages should include:

- Publisher signature
- Package hash
- Manifest hash
- Compatibility range
- Revocation metadata

Unsigned local plugins may be allowed only behind a developer-mode warning.

## Plugin data

Each plugin receives a private storage namespace. It must not access:

- Other plugin data
- Raw application database
- Unrelated source credentials
- Private notes
- Viewing history unless a future capability explicitly requires it

## Resource limits

- Memory limit
- CPU time limit
- Request timeout
- Maximum concurrent requests
- Maximum response size
- Maximum cached plugin data
- Crash threshold

Repeated crashes trigger automatic disablement.

## Developer tooling

Provide:

- Connector SDK
- Schema types
- Mock HTTP server
- Fixture recorder with redaction
- Contract test runner
- Manifest validator
- Permission simulator
- Local registry
- Compatibility checker

## Review checklist

- Uses only declared domains
- Does not log credentials
- Honors rate limits
- Correctly distinguishes stream and download
- Preserves source attribution
- Handles deleted content
- Returns structured errors
- Includes tests
- Includes license
