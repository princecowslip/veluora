# Operations and Observability

## Local-first observability

Most diagnostics remain on the device.

Metrics:

- Index queue depth
- items indexed
- thumbnail queue depth
- search latency
- connector latency
- connector failures
- rate-limit events
- playback failures
- download throughput
- cache size
- database size
- migration status
- plugin crashes

## Health states

### Application

- Healthy
- Degraded
- Repair required
- Safe mode

### Source

- Healthy
- Authentication required
- Rate limited
- Schema mismatch
- Network unavailable
- Disabled
- Revoked

### Storage

- Healthy
- Near quota
- Full
- Read-only
- Integrity warning

## Logging levels

- Error
- Warning
- Info
- Debug
- Trace

Info should be privacy-safe. Debug and trace require explicit temporary activation.

## Redacted support bundle

May include:

- Application version
- Operating system
- Database schema version
- Enabled connector IDs and versions
- Permission manifests
- Error codes
- redacted logs
- health-check summaries
- dependency versions

Must exclude:

- Credentials
- titles
- descriptions
- search terms
- tags
- notes
- full URLs
- local paths
- thumbnails
- media

## Crash handling

- Save minimal crash metadata locally.
- Offer restart in safe mode.
- Disable recently crashing plugin.
- Preserve download and indexing state.
- Never upload automatically.
- Let the user inspect what will be sent.

## Background jobs

Job classes:

- Foreground
- Interactive
- Background
- Maintenance

Priority:

1. Playback and reader fetch
2. User-triggered metadata
3. Search
4. Indexing
5. Downloads
6. Fingerprinting
7. Cache maintenance

## Backup and restore

Backup includes:

- Database
- configuration
- local metadata overrides
- collections
- progress
- block rules
- plugin list

Credentials are excluded or exported separately through a secure mechanism.

Restore process:

1. Validate archive.
2. Check schema compatibility.
3. Create current-profile backup.
4. Import into staging database.
5. Validate integrity.
6. Swap atomically.
7. Rebuild regenerable indexes and thumbnails.
