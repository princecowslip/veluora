# Testing Strategy

## Test pyramid

### Unit tests

Cover:

- Query parsing
- query translation
- tag normalization
- progress rules
- capability checks
- URL validation
- path sanitization
- block rules
- file naming
- duplicate scoring
- download eligibility
- permission evaluation

### Integration tests

Cover:

- SQLite repositories
- filesystem indexing
- credential-store adapter
- media probe
- thumbnail generation
- local API
- connector host
- download manager

### Contract tests

Every connector runs the same suite:

- Capability response
- successful search
- empty search
- pagination
- authentication required
- invalid credentials
- rate limit
- deleted item
- expired media URL
- malformed metadata
- unsupported filter
- network timeout
- source schema change fixture

### End-to-end tests

Key flows:

- First-run onboarding
- Add library folder
- index mixed media
- search
- play and resume
- read and resume
- favorite and collect
- block a tag
- clear history
- add a source
- handle source failure
- download authorized media
- export and restore profile

## Test fixture library

Use safe synthetic fixtures containing:

- JPEG, PNG, GIF, WebP — the four formats the `image` crate is actually
  configured to decode (`Cargo.toml`); AVIF is not decoded anywhere
- animated GIF and WebP
- MP4, WebM, MKV
- MP3, AAC, FLAC, Opus
- plain text, Markdown — shipped story formats; sanitized HTML and EPUB
  remain unbuilt (`KNOWN_ISSUES.md`)
- ZIP comic archives
- right-to-left page order
- corrupt media
- extremely large declared dimensions
- mismatched MIME types
- archive traversal attempts
- decompression bombs represented by safe test simulators

## Performance tests

Measure:

- Startup
- first local result
- 100,000-item search
- 1,000,000-tag index
- thumbnail queue throughput
- duplicate scan
- library rescan
- concurrent connector search
- download throughput
- database migration

## Security tests

- Secret redaction
- local API cross-origin attack
- DNS rebinding
- malicious redirect
- private-network access from plugin
- oversized response
- archive traversal
- decompression abuse
- media decoder crash containment
- plugin permission escalation
- unsigned plugin behavior
- lock-screen data leakage
- operating-system recent-window thumbnail leakage

## Accessibility tests

- Keyboard-only navigation
- screen reader labels
- focus order
- high contrast
- 200% zoom
- reduced motion
- caption selection
- monochrome TUI
- plain CLI output

## Privacy deletion tests

After each deletion action, verify:

- Database rows
- FTS rows
- thumbnails
- cache
- logs
- temporary files
- operating-system recent files where controllable
- credentials
- exports
- background worker memory after restart

## Release gates

A release cannot ship with:

- Known credential leakage
- known local API exposure
- corrupting migration
- broken data deletion
- uncontained plugin crash
- archive traversal
- unrestricted remote binding
- disabled safety blocks
