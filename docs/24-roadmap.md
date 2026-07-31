# Roadmap and Milestones

## Phase 0: Definition and risk control

Deliverables:

- Product requirements
- prohibited-content policy
- threat model
- normalized data model
- supported-format list
- connector governance policy
- architecture decision records

Exit criteria:

- Non-goals and safety boundaries approved.
- Data categories have retention rules.
- MVP operating systems selected.

## Phase 1: Core local library

Deliverables:

- Database and migrations
- folder indexing
- media probing
- local item model
- favorites
- collections
- progress
- basic local search
- CLI foundation

Exit criteria:

- Mixed local library can be indexed and queried.
- User state survives restart and file moves where hashes match.

## Phase 2: Media experience

Deliverables:

- Image viewer
- video player integration
- audio player
- story reader
- comic reader
- thumbnails
- external player adapters

Exit criteria:

- Every supported media type opens and resumes.
- Malformed media failures are contained.

## Phase 3: Desktop GUI

Deliverables:

- App shell
- Home
- Library
- item details
- players and readers
- collections
- settings
- lock and private mode

Exit criteria:

- A non-technical user can complete all local-library workflows.

## Phase 4: Privacy, storage, and diagnostics

Deliverables:

- Privacy Center
- history retention
- independent deletion
- cache quotas
- encrypted metadata option
- support bundle
- safe mode

Exit criteria:

- Deletion tests pass.
- No explicit data appears in default logs or notifications.

## Phase 5: Connector framework

Deliverables:

- Connector SDK
- plugin host
- capability model
- official API reference connector
- feed connector
- generic booru-compatible connector
- source setup wizard

Exit criteria:

- Source failure cannot crash the main application.
- Unsupported features are represented explicitly.

## Phase 6: Unified search

Deliverables:

- query AST — delivered (`domain::search_query`, Milestone B)
- translation — delivered (`SourceService`'s per-connector unsupported-clause reporting, Milestone F)
- progressive source results — delivered (`application::discover::DiscoverService`, Milestone I: per-source results and failures are isolated and reported independently, not blocked on the slowest/broken source)
- source diagnostics — delivered (`DiscoverSourceStatus` per source, Milestone I)
- duplicate collapsing — not delivered; the same underlying media appearing via two connectors shows as two separate Discover hits (see `KNOWN_ISSUES.md`)
- saved searches — not delivered

Exit criteria:

- Results stream independently. — met for isolation (a broken source can't block the others); not met for true streaming (Discover's HTTP response is a single synchronous aggregate, not an incremental/event stream).
- Private local metadata is never transmitted. — met: Discover only ever calls out to a source's own connector with the raw query text, never local-only fields like notes or private tags.

## Phase 7: Downloads and offline mode

Deliverables:

- download state machine — delivered (`domain::DownloadState`/`application::download::DownloadService`, Milestone J)
- resume — delivered (`Range`/`If-Range`-based resume, restarting from byte zero when the source's content changed or ignored the range; see `KNOWN_ISSUES.md` for what isn't covered — e.g. no automatic resume-on-restart without a daemon)
- verification — delivered architecturally (a `blake3` checksum is always computed and compared when a source declares one), but not exercised end-to-end yet — `FeedConnector` (the only download-capable connector) has no checksum field to declare
- quotas — delivered (`PrivacyService::enforce_download_quota`, sharing its eviction loop with the existing cache-quota policy)
- cache policy — delivered for the "never evict pinned" rule; not delivered for `docs/17-downloads-cache-storage.md`'s other listed eviction policies (least-recently-opened, oldest-unviewed, per-source limits)
- offline indicators — not delivered; there's no UI surface distinguishing "available offline" from "requires the source" beyond `media_variants.local_path` being set or not

Exit criteria:

- Download is shown only for authorized variants. — met: `DownloadService::check_eligibility` gates on `download_permitted` (ADR-007), a downloads-capable connector, an enabled source, and no matching block rule, and every UI surface (GUI's Viewer, CLI's `download add`, `local-api`'s `POST /downloads`) goes through it.
- Interrupted downloads recover safely. — met: a temp file is only ever promoted to the final path via an atomic rename after the transfer (and any declared checksum) succeeds, so a partial or corrupted download never appears complete; see `KNOWN_ISSUES.md` for what's still out of scope.

## Phase 8: Terminal UI

Deliverables:

- responsive panes
- search
- library navigation
- details
- queue
- downloads
- external playback
- private mode

Exit criteria:

- Core workflows operate over SSH without graphics.

## Phase 9: Third-party plugins

Deliverables:

- signed packages
- registry
- permission prompts
- revocation
- developer toolkit

Exit criteria:

- Plugins cannot exceed declared permissions.
- Permission changes require user approval.

## Phase 10: Optional remote mode

Separate project gate requiring:

- legal review
- authenticated TLS
- device authorization
- age-assurance strategy
- audit events
- regional controls
- hosted safety operations

Remote mode should not delay a strong local product.
