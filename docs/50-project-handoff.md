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

- [ ] Core language chosen.
- [ ] GUI toolkit chosen.
- [ ] Local API or IPC chosen.
- [ ] Database migration framework chosen.
- [ ] Credential-store adapters selected.
- [ ] Media engine selected.
- [ ] CI matrix implemented.
- [ ] notcurses package strategy confirmed.

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
- Specifications for a CMake TUI scaffold and JSON/CSS design assets (not yet generated — see the "planned" notes in docs 30, 41, 42, 43, 45, and 49)
- Implementation and traceability plans
