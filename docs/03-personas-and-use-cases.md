# Personas and Use Cases

## Persona 1: Private local collector

### Context

Maintains a large collection across folders, drives, and archive formats. Values organization and privacy more than online discovery.

### Needs

- Fast local indexing
- Duplicate detection
- Metadata correction
- Collections and private tags
- Reliable resume state
- Backup and export
- Neutral filenames and notifications

### Primary journey

1. Select folders during setup.
2. Preview detected formats and estimated index size.
3. Start background indexing.
4. Browse a unified local library.
5. Fix unmatched titles or creators.
6. Group duplicates.
7. Build collections and saved searches.
8. Export metadata backup.

## Persona 2: Multi-source browser

### Context

Uses several lawful sites and archives with different search syntax and media formats.

### Needs

- One search field
- Source filtering
- Clear attribution
- Authentication management
- Consistent viewer behavior
- Rate-limit transparency
- Source health status

### Primary journey

1. Add approved sources.
2. Review requested permissions.
3. Sign in where needed.
4. Search all enabled sources.
5. Compare variants.
6. Open the original page or play through an authorized stream.
7. Save a local reference or download when permitted.

## Persona 3: Keyboard-first user

### Context

Prefers terminal workflows, scripts, and external players.

### Needs

- Stable CLI
- JSON output
- Shell completion
- TUI over SSH
- External viewer and player commands
- Predictable exit codes
- Idempotent commands

### Primary journey

```bash
veloura search 'type:audio language:en -tag:blocked' --output json
veloura queue add ITEM_ID
veloura item play ITEM_ID
veloura collection add ITEM_ID --to COLLECTION_ID
```

## Persona 4: Privacy-sensitive user

### Context

Shares a device or wants strict separation between ordinary activity and sensitive media.

### Needs

- Application lock
- Private sessions
- Neutral interface mode
- Separate cache and history deletion
- No telemetry
- Encrypted metadata
- Configurable thumbnail storage
- Panic shortcut

### Primary journey

1. Enable encrypted metadata.
2. Configure operating-system authentication.
3. Use a private session.
4. Hide explicit thumbnails until unlock.
5. Exit with a panic shortcut.
6. Clear the private session automatically.

## Persona 5: Connector maintainer

### Context

Maintains a connector for an official API or compatible source.

### Needs

- Stable connector contract
- Mock server
- Contract tests
- Capability declarations
- Rate-limit helpers
- Version compatibility rules
- Safe credential APIs
- Clear release process

## Core use cases

### Discover an item across sources

- Enter a query.
- Select media types and sources.
- See partial results as sources respond.
- Receive a warning for failed or unsupported sources.
- Refine with local filters.
- Open details with attribution and available actions.

### Continue reading or playback

- Open Continue from the home screen.
- Resume from saved position.
- Update progress periodically.
- Mark complete near the end according to media type.
- Return to the next chapter or related item.

### Block unwanted material

- Open item actions.
- Block a tag, creator, source, series, or exact item.
- Preview how many existing items will be hidden.
- Apply globally.
- Allow review and reversal in settings.

### Clear sensitive data

- Open Privacy Center.
- Choose history, searches, thumbnails, cache, downloads, credentials, or all local data.
- Show impact before deletion.
- Perform deletion.
- Verify removal and report failures.

### Diagnose a source

- Open Sources.
- Select a degraded connector.
- Run health check.
- Review authentication, rate limit, schema, and media URL status.
- Export a redacted diagnostic bundle.
