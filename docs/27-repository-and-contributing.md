# Repository Structure and Contribution Guide

## Proposed repository layout

```text
/
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── docs/
├── crates-or-packages/
│   ├── domain/
│   ├── application/
│   ├── database/
│   ├── media/
│   ├── search/
│   ├── connectors/
│   │   ├── local-files/
│   │   ├── feed/
│   │   ├── official-example/
│   │   └── booru-compatible/
│   ├── plugin-host/
│   ├── local-api/
│   ├── cli/
│   └── gui/
├── tui/
│   ├── CMakeLists.txt
│   ├── CMakePresets.json
│   ├── include/
│   └── src/
├── assets/
│   └── dependencies.json
├── fixtures/
├── migrations/
├── scripts/
├── packaging/
└── tests/
```

## Branch policy

- Main is releasable.
- Features use short-lived branches.
- Security fixes may use private branches.
- Connector updates may release independently when compatible.

## Pull request requirements

- Clear problem statement
- Scope and non-goals
- Tests
- migration notes
- privacy impact
- security impact
- connector permission changes
- screenshots for GUI changes
- terminal captures for TUI changes
- CLI output examples
- documentation updates

## Coding standards

- Treat warnings as errors in CI where practical.
- Use typed errors.
- Avoid logging unredacted external data.
- Keep network access behind connector interfaces.
- Pass external process arguments without shell interpolation.
- Add cancellation to long-running operations.
- Use bounded concurrency.
- Document public APIs.

## Connector contributions

A connector pull request must include:

- Manifest
- Supported domains
- source authorization basis
- capability list
- authentication description
- rate-limit behavior
- download policy
- fixtures
- contract tests
- deletion handling
- license

## Safety review

Changes require additional review when they:

- Add a source
- expand domains
- add browser-cookie access
- enable filesystem writes
- alter block behavior
- change age or safety controls
- add remote access
- add telemetry
- change deletion behavior

## Issue templates

- Bug
- Feature request
- Connector failure
- Source policy concern
- Privacy issue
- Security vulnerability
- Accessibility issue

Security vulnerabilities should use a private reporting route, not a public issue.

## Documentation style

- Use neutral terminology.
- Avoid embedding explicit imagery.
- Use synthetic examples.
- Include source and privacy implications.
- Keep commands copyable.
- Mark proposed versus accepted decisions.
