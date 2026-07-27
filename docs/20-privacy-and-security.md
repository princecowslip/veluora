# Privacy and Security

## Threat model

Protected assets:

- Viewing history
- Search history
- Favorites and collections
- Private notes and tags
- Thumbnails
- Downloaded media
- Source credentials
- Source membership
- Local file paths
- Diagnostic logs

Potential attackers:

- Another local device user
- Malicious plugin
- Compromised source
- Network observer
- Malicious media file
- Browser-origin attack against the local API
- Malware with user-level access

## Privacy defaults

- No account required.
- Telemetry off.
- Crash upload off.
- Local API bound to loopback.
- Explicit notifications off.
- Search history uses limited retention.
- Application starts locked on shared-device profiles.
- Browser-cookie import disabled.
- Remote access disabled.

## Credential handling

- Use the operating-system credential manager.
- Assign credentials to one source.
- Provide revoke and test operations.
- Do not include secrets in configuration export.
- Redact authorization headers and cookies.
- Avoid passing secrets through command arguments or environment variables where possible.

## Database encryption

This is not yet decided; see the "Encryption" open question in [48 — Open Questions and Decisions](48-open-questions-and-decisions.md), which tracks the choice of library, default-vs-optional behavior, key storage, and recovery strategy.

Whatever approach is chosen, the product should document what remains visible, such as filenames and unencrypted downloads.

## Private sessions

A private session should:

- Avoid writing search history.
- Avoid writing viewing history, unless the user explicitly saves progress.
- Store temporary cookies in memory where possible.
- Clear temporary thumbnails and responses on exit.
- Hide activity from Continue and Recent sections.
- Lock automatically after timeout.

## Application lock

Unlock methods:

- Operating-system authentication
- Profile password
- Hardware-backed credential where supported

Do not invent custom cryptography for password storage.

## Network security

- HTTPS by default.
- Certificate validation.
- Strict redirects.
- Private-address blocking for untrusted connectors.
- DNS rebinding defenses for local API.
- Request and response limits.
- Safe decompression.
- Per-source cookie jars.
- No ambient proxy credentials for plugins without permission.

## Media security

- MIME sniffing and validation
- Decoder isolation
- archive traversal prevention
- decompression limits
- maximum dimensions
- timeouts
- temporary directory isolation
- no execution of embedded scripts
- metadata stripping for generated previews

## Logging

Default logs should contain:

- Event code
- Component
- Timestamp
- Redacted source ID
- Technical error category

Default logs should not contain:

- Titles
- Search queries
- URLs with query strings
- file paths
- tags
- credentials
- notes

A verbose diagnostic mode may include additional data only after warning and should generate a redacted support bundle.

## Security update process

- Signed releases
- Dependency scanning
- vulnerability disclosure policy
- connector revocation
- plugin signature verification
- database backup before migrations
- rollback path
- public security advisories
