# Project Handoff Checklist

## Documentation

- [ ] README reflects the current product name.
- [ ] Documentation index is generated.
- [ ] Internal links pass.
- [ ] JSON assets parse.
- [ ] Commands are clearly marked as examples or verified commands.
- [ ] Proposed and accepted decisions are distinguished.
- [ ] External source presets are marked by integration tier.
- [ ] Safety and privacy boundaries are visible.
- [ ] No real explicit test content is embedded.

## Product

- [ ] Product owner assigned.
- [ ] MVP scope approved.
- [ ] Tier 1 platforms selected.
- [ ] Bundled source list approved.
- [ ] Hosted or remote mode decision recorded.
- [ ] Success measures approved.

## Design

- [ ] Brand name undergoes clearance.
- [ ] Logo direction selected.
- [ ] Design tokens imported into the chosen GUI system.
- [ ] Home, Library, and Viewer prototypes reviewed.
- [ ] Blur and locked-state behaviour tested with users.
- [ ] Accessibility review scheduled.

## Engineering

- [x] Core language chosen — Rust (workspace since Milestone A).
- [x] GUI toolkit chosen — iced 0.13 (Milestone D).
- [x] Local API or IPC chosen — loopback-only HTTP `local-api` (Milestone A).
- [x] Database migration framework chosen — versioned SQL files under
  `migrations/`, applied by a runner in `crates/database` (Milestone A).
- [ ] Credential-store adapters selected — not yet; connector credentials
  (e.g. booru API keys, OPDS Basic-auth passwords) are stored in plain
  `configuration_json` today, no OS-credential-manager adapter exists.
- [x] Media engine selected — external FFmpeg (`ffprobe`/`ffmpeg` on `PATH`)
  for probing/thumbnailing, external player launch for playback
  (Milestone C).
- [x] CI matrix implemented — GitHub Actions builds and tests the Rust
  workspace and the C++ TUI on every push.
- [x] notcurses package strategy confirmed — packaged installation via
  `scripts/install-tui-deps.sh`, documented in
  [45 — Required Packages and Dependencies](45-required-packages-dependencies.md).

## Security and privacy

- [ ] Threat model reviewed.
- [ ] Secret-handling design approved.
- [ ] Local API binding and authentication approved.
- [ ] Plugin sandbox design approved.
- [ ] Deletion-verification plan approved.
- [ ] Logging fields reviewed.
- [ ] Vulnerability disclosure route published.

## Source governance

- [ ] Connector review template created.
- [ ] Source terms reviewed.
- [ ] Rate limits documented.
- [ ] Download capability documented.
- [ ] Revocation mechanism designed.
- [ ] Browser handoff templates reviewed.
- [ ] Metadata providers reviewed.

## Delivery

- [ ] Initial backlog created from the implementation plan.
- [ ] Requirements receive identifiers.
- [ ] Milestones have owners.
- [ ] Definition of done is adopted.
- [ ] Release gates are automated where possible.
- [ ] Changelog process is established.
- [ ] Support and diagnostics ownership is assigned.

## Handoff package contents

The final planning package includes:

- Product requirements
- UX and visual design
- GUI, TUI, and CLI specifications
- Architecture and data model
- Connector and plugin design
- Search, taxonomy, layout, and blur policies
- Privacy, security, and safety requirements
- Testing and release gates
- Dependency inventory
- A CMake TUI scaffold — built and CI-tested, not just specified; see `tui/`
- Design/documentation JSON assets — `assets/dependencies.json` exists;
  `colors.json`, `colors.css`, `source-presets.json`, `taxonomy.json`, and
  `ui-preferences.json` remain planned, see the "still planned" notes in
  docs 30, 41, 42, 43, and 49
- Implementation and traceability plans
