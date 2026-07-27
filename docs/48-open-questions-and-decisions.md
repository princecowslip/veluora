# Open Questions and Decisions

## Decisions already proposed

- Product name: Veloura
- Default visual direction: Midnight Gallery
- Main canvas: pure black
- Primary accent: indigo
- TUI: C++20 with notcurses 3.x
- Local database: SQLite with FTS5
- Interface architecture: shared local application service
- Public connectors: disabled by default
- Public Home feeds: explicit opt-in
- Third-party plugins: isolated and permissioned
- Downloads: explicit source capability required
- Browser-only sources: handoff templates, not scrapers

These remain proposed until recorded as accepted ADRs.

## Decisions required before implementation

### Supported operating systems

Choose:

- Linux distributions and minimum versions
- macOS minimum version
- Windows support level
- FreeBSD support level
- CPU architectures

### Core language and framework

Choose:

- Core service language
- Desktop GUI toolkit
- IPC protocol
- Packaging framework
- Update mechanism

### Encryption

Decide:

- Database encryption library or approach — candidates: standard SQLite with operating-system disk encryption, SQLCipher or equivalent, a separate encrypted user-state database, or full profile encryption
- Default or optional encryption
- Key storage
- Recovery strategy
- What remains visible outside encryption (for example filenames and unencrypted downloads)

Referenced from [20 — Privacy and Security](20-privacy-and-security.md) and ADR-003 in [26 — Architecture Decisions](26-architecture-decisions.md).

### Media playback

Decide:

- Embedded libmpv
- Controlled external mpv
- Platform player adapters
- Subtitle rendering ownership
- Hardware decoding defaults

### Connector distribution

Decide:

- Built-in connector list
- Official registry governance
- Review ownership
- Revocation mechanism
- Connector release cadence

### Metadata providers

Decide:

- Automatic matching threshold
- Manual review threshold
- Allowed provider fields
- Conflict resolution
- Performer and creator identity policy

### Remote access

Decide whether remote access is:

- Permanently separate
- A later optional module
- A headless-server edition
- Unsupported

### Recommendation system

Decide:

- Whether recommendations ship
- Local-only algorithm
- Explainability
- Reset controls
- Private-session behaviour
- Sensitive category treatment

### Package licensing

Review:

- Application license
- Connector licenses
- notcurses license
- FFmpeg build configuration
- mpv licensing
- Codec distribution
- Static versus dynamic linking

## Release-blocking questions

Before an MVP release, answer:

1. Which exact file formats are guaranteed?
2. Which sources are bundled?
3. Which operating systems are Tier 1?
4. How is the local service authenticated?
5. How are credentials stored on each operating system?
6. How is complete deletion verified?
7. What is the minimum terminal capability for `veloura-tui`?
8. Which dependency versions are pinned?
9. What is the vulnerability disclosure contact?
10. Which legal and policy reviews have been completed?

## Decision process

For each material choice:

1. Create an ADR.
2. Document alternatives.
3. Record privacy, security, accessibility, maintenance, and packaging consequences.
4. Approve the decision.
5. Update affected documents.
6. Add tests or release gates.
