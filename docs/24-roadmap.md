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

- query AST
- translation
- progressive source results
- duplicate collapsing
- source diagnostics
- saved searches

Exit criteria:

- Results stream independently.
- Private local metadata is never transmitted.

## Phase 7: Downloads and offline mode

Deliverables:

- download state machine
- resume
- verification
- quotas
- cache policy
- offline indicators

Exit criteria:

- Download is shown only for authorized variants.
- Interrupted downloads recover safely.

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
