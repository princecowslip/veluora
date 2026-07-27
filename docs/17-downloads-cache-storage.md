# Downloads, Cache, and Storage

## Storage classes

### Metadata

- Database
- Search indexes
- Local overrides
- Progress
- Collections
- Block rules

### Regenerable cache

- Thumbnails
- Preview images
- Temporary story renderings
- Temporary source responses
- Expiring media segments

### Permanent downloads

User-requested files saved only when allowed by the source.

### Temporary files

- Partial downloads
- Archive extraction
- Media probing
- Plugin exchange files

Temporary files must be cleaned after crashes where possible.

## Directory layout

```text
config/
database/
cache/
  thumbnails/
  previews/
  responses/
downloads/
plugins/
logs/
temp/
```

## Download eligibility

Before showing Download:

- Connector capability says downloading is supported.
- Item variant marks download as permitted.
- Required authentication is present.
- Destination is writable.
- Storage policy allows the estimated size.

## Download state machine

```text
Queued
→ Resolving
→ Downloading
→ Verifying
→ Finalizing
→ Complete
```

Failure states:

- Paused
- Authentication required
- Rate limited
- Source removed
- Insufficient space
- Checksum failed
- Permission revoked
- Cancelled

## Resume

Resume only when:

- Server supports ranges or connector provides a resume mechanism.
- Partial file matches expected item and variant.
- Authorization remains valid.
- Content has not changed.

## File naming

Use sanitized templates:

```text
{creator}/{series}/{title} [{source_id}]/{sequence}.{ext}
```

Requirements:

- Remove traversal and reserved characters.
- Enforce maximum length.
- Handle duplicate names.
- Avoid explicit filenames in neutral mode if the user selects opaque naming.
- Store display title in metadata rather than relying on filename.

## Quotas

Separate quotas for:

- Thumbnails
- General cache
- Temporary segments
- Permanent downloads

Policies:

- Least recently used
- Oldest unviewed
- Source-specific limit
- Never remove pinned
- Never remove permanent downloads automatically unless explicitly configured

## Integrity

- Record file size.
- Use source checksum when provided.
- Compute a local cryptographic hash after completion.
- Verify archive readability.
- Mark unverified rather than pretending success.

## Encryption

Options:

- Encrypted database
- Encrypted download directory through operating-system or user-managed storage
- Encrypted thumbnail cache
- Opaque file naming

Application-level media encryption adds key-management and playback complexity and should be optional, not improvised.

## Deletion

Deletion controls must distinguish:

- Remove from library only
- Delete cached copy
- Delete permanent file
- Delete source reference
- Delete all local metadata
- Clear history but retain item

Destructive file deletion should require clear confirmation.
