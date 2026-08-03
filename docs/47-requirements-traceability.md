# Requirements Traceability Matrix

## Purpose

This matrix connects major product requirements to design documents, implementation areas, and verification evidence.

The "Verification" column names the intended verification *method* for
each requirement — it does not assert that verification has already run.
In particular, per `KNOWN_ISSUES.md`: no automated accessibility testing
exists yet (no iced accessibility-testing harness), styling is
functional-first rather than pixel-accurate to `docs/52-sample-ui-spec.md`,
and no packaging scripts exist yet — so "Screenshot and contrast review,"
"Keyboard and assistive tests," and "Package installation test" below
describe what should eventually happen, not evidence already collected.

| Requirement | Primary documents | Implementation area | Verification |
|---|---|---|---|
| Local-first operation | 01, 02, 12, 20 | Core service, database, filesystem | Offline end-to-end tests |
| GUI, TUI, and CLI parity | 08, 09, 10, 12 | Application services and local API | Cross-interface contract tests |
| Video, image, animation, story, audio, manga, comic support | 04, 16, 34 | Media adapters and viewers | Media fixture suite |
| Private local metadata | 01, 13, 20 | SQLite and credential store | Privacy inspection and deletion tests |
| Pure-black visual system | 07, 29, 30 | GUI and TUI themes | Screenshot and contrast review |
| Home feed is opt-in for public sources | 06, 08, 52, 36 | Feed service and settings | Feed eligibility tests |
| Source capability visibility | 02, 14, 41 | Connector runtime and UI | Connector contract tests |
| No DRM or access-control bypass | 01, 02, 21 | Connector and download policy | Negative capability tests |
| Downloads only when permitted | 17, 21, 34 | Download eligibility policy | Connector and UI tests |
| Orientation and identity are not inferred | 13, 42 | Metadata and taxonomy mapping | Mapping review and unit tests |
| User-controlled categories and tags | 15, 34, 42, 44 | Search and taxonomy service | Query and autocomplete tests |
| Configurable blur and reveal | 07, 52, 43 | Visibility policy and rendering | Locked, source, tag, and item tests |
| Blocked content does not render | 06, 20, 21, 43 | Safety filter before presentation | Pre-render policy tests |
| Credentials remain source-scoped | 14, 18, 20 | Credential manager and plugin host | Secret-scope tests |
| Local API is protected | 19, 20 | IPC or loopback server | Origin, token, and binding tests |
| notcurses TUI | 09, 45 | C++20 TUI client | Terminal matrix and shutdown tests |
| Search history is controllable | 06, 15, 20, 37 | Privacy and search services | Retention and private-session tests |
| Complete data deletion | 20, 22, 28, 39 | Privacy service and repositories | Post-deletion verification suite |
| Accessible keyboard use | 06, 08, 09, 11 | All interfaces | Keyboard and assistive tests |
| Plugins use explicit permissions | 18, 20 | Plugin host | Permission and escalation tests |
| Connector failure isolation | 12, 14, 22 | Worker and connector host | Crash and timeout tests |
| Stable scripting interface | 10, 19 | CLI and local API | Schema and exit-code tests |
| Upgrade safety | 13, 22, 23, 28 | Migrations and backup | Upgrade and rollback tests |

## Requirement identifiers

Implementation issues should use identifiers such as:

```text
REQ-PRIV-001
REQ-SOURCE-004
REQ-TUI-012
REQ-MEDIA-008
REQ-A11Y-003
```

Recommended prefixes:

- `REQ-PROD`
- `REQ-PRIV`
- `REQ-SAFE`
- `REQ-SOURCE`
- `REQ-MEDIA`
- `REQ-SEARCH`
- `REQ-GUI`
- `REQ-TUI`
- `REQ-CLI`
- `REQ-A11Y`
- `REQ-OPS`
- `REQ-PLUGIN`

## Evidence policy

A requirement is not considered verified by documentation alone.

Acceptable evidence includes:

- Automated test
- Manual test record
- Accessibility review
- Security review
- Migration rehearsal
- Screenshot or terminal capture
- Package installation test
- Source connector contract result
