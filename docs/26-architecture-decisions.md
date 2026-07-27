# Architecture Decisions

This file contains the accepted architecture decision records for the planning package. Each decision should be moved to an individual ADR file as implementation begins.

## ADR-001: Local-first metadata

**Status:** Accepted

**Decision:** Store library metadata and user state locally by default.

**Rationale:**

- Protects privacy.
- Keeps local library usable offline.
- Avoids requiring an account.
- Simplifies early compliance scope.

**Consequences:**

- Multi-device sync is deferred.
- Users must manage backups.
- Remote mode requires a separate design.

## ADR-002: Shared core for GUI, TUI, and CLI

**Status:** Accepted

**Decision:** Implement business logic in shared application services.

**Rationale:**

- Prevents inconsistent capability and safety behavior.
- Makes CLI useful for integration testing.
- Reduces duplicated source and database logic.

**Consequences:**

- Presentation interfaces must use stable service boundaries.
- UI-specific shortcuts remain outside the core.

## ADR-003: SQLite for local metadata

**Status:** Accepted

**Decision:** Use SQLite with migrations and full-text search.

**Rationale:**

- Mature, portable, transactional, and appropriate for local applications.
- Supports large libraries with correct indexing.
- Easy backup and inspection.

**Consequences:**

- Large media stays in the filesystem.
- Concurrency must be designed around SQLite behavior.
- Database encryption approach is not yet decided; see the consolidated open question in [48 — Open Questions and Decisions](48-open-questions-and-decisions.md).

## ADR-004: Capability-based connectors

**Status:** Accepted

**Decision:** Connectors declare supported operations and query features.

**Rationale:**

- Sources vary widely.
- Prevents pretending an operation is universally available.
- Enables clear UI explanations.

**Consequences:**

- Application code must handle partial support.
- Connector tests need capability matrices.

## ADR-005: No unrestricted HTML scraping engine

**Status:** Accepted

**Decision:** Official connectors target documented APIs, feeds, or narrowly defined source integrations.

**Rationale:**

- Reduces maintenance, legal, and security risk.
- Avoids creating a general circumvention tool.
- Supports predictable testing.

**Consequences:**

- Fewer supported sources.
- Community demand may exceed official scope.
- Connector governance becomes important.

## ADR-006: Third-party plugins run outside the main process

**Status:** Accepted

**Decision:** Isolate plugins with explicit permissions and resource limits.

**Rationale:**

- Connectors process untrusted network data.
- Plugins must not receive database or credential access.
- Crashes should not terminate the application.

**Consequences:**

- More IPC and packaging complexity.
- Some plugin APIs will be less flexible.

## ADR-007: Downloads require explicit source capability

**Status:** Accepted

**Decision:** A playable stream is not considered downloadable unless the source marks it so.

**Rationale:**

- Separates playback access from copying rights.
- Keeps UI behavior aligned with source permissions.

**Consequences:**

- Some technically downloadable streams will not show a download action.
- Connectors need accurate capability metadata.

## ADR-008: Sensitive text excluded from default logs

**Status:** Accepted

**Decision:** Default logs use identifiers and error codes, not titles, queries, tags, or local paths.

**Rationale:**

- Logs are frequently shared.
- Explicit metadata can be more sensitive than technical state.

**Consequences:**

- Diagnostics may require temporary verbose mode.
- Support tools need controlled redaction.
