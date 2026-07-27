# Documentation Index

This is the canonical index for the Veloura planning package.

## Foundation

- [00 — Glossary](00-glossary.md)
- [01 — Product Vision](01-product-vision.md)
- [02 — Scope and Requirements](02-scope-and-requirements.md)
- [03 — Personas and Use Cases](03-personas-and-use-cases.md)
- [04 — Feature Catalogue](04-feature-catalogue.md)

## Experience design

- [05 — Information Architecture](05-information-architecture.md)
- [06 — UI and UX Principles](06-ui-ux.md)
- [07 — Visual Design System](07-visual-design-system.md)
- [08 — Desktop GUI Specification](08-desktop-gui.md)
- [09 — Terminal UI Specification — notcurses](09-terminal-ui.md)
- [10 — CLI Specification](10-cli.md)
- [11 — Accessibility](11-accessibility.md)

## Architecture and engineering

- [12 — System Architecture](12-system-architecture.md)
- [13 — Domain and Data Model](13-data-model.md)
- [14 — Source Connector Framework](14-source-connectors.md)
- [15 — Search and Discovery](15-search-and-discovery.md)
- [16 — Media Handling](16-media-handling.md)
- [17 — Downloads, Cache, and Storage](17-downloads-cache-storage.md)
- [18 — Plugin System](18-plugin-system.md)
- [19 — Local API](19-local-api.md)

## Privacy, safety, testing, and operations

- [20 — Privacy and Security](20-privacy-and-security.md)
- [21 — Content Safety and Compliance](21-content-safety-and-compliance.md)
- [22 — Testing Strategy](22-testing-strategy.md)
- [23 — Operations and Observability](23-operations-and-observability.md)

## Roadmap, governance, and brand

- [24 — Roadmap and Milestones](24-roadmap.md)
- [25 — Backlog and Acceptance Criteria](25-backlog.md)
- [26 — Architecture Decisions](26-architecture-decisions.md)
- [27 — Repository Structure and Contribution Guide](27-repository-and-contributing.md)
- [28 — Release Checklist](28-release-checklist.md)
- [29 — Brand, Source Presets, and Aesthetic](29-brand-source-presets.md)

## Design assets and UI specifications

- [30 — Design Tokens and UI Colour Roles](30-design-tokens.md)
- [31 — Veloura Logo Concept Sheet](31-logo-concept-sheet.md) — four original concepts, including "Veiled Bookmark"
- [32 — UI/UX Wireframes](32-ui-wireframes.md)
- [33 — Veloura Logo Concept Sheet, revised direction](33-logo-concept-sheet.md) — refined geometry and a recommended concept; does not fully supersede 31 (drops "Veiled Bookmark")
- [51 — Sample Mock UI Specification](51-sample-mock-ui-spec.md) — ASCII wireframe mock, no pixel measurements
- [52 — Sample UI Specification](52-sample-ui-spec.md) — exact pixel measurements, no ASCII wireframes

Docs 32, 51, and 52 each describe the same Home/Library/Viewer screens at a different level of detail; they are complementary, not contradictory. (Docs 51 and 52 previously shared the numbers 30 and 31 with 30-design-tokens.md and 31-logo-concept-sheet.md respectively — they were renumbered to remove the collision.)

## Features and product polish

- [34 — Features and Functions](34-features-and-functions.md)
- [35 — User-Friendly Workflows](35-user-friendly-workflows.md)
- [36 — Home Feed and Personalization](36-home-feed-and-personalization.md)
- [37 — Settings and Preferences](37-settings-and-preferences.md)
- [38 — Feedback, Errors, and Notifications](38-feedback-errors-and-notifications.md)
- [39 — Release Polish Checklist](39-release-polish-checklist.md)
- [40 — Final Product Summary](40-final-product-summary.md)

## Sources, taxonomy, visibility, and dependencies

- [41 — Expanded Source and Site Catalogue](41-expanded-source-catalogue.md)
- [42 — Categories, Tags, and Orientation Taxonomy](42-categories-tags-orientation.md)
- [43 — Layout, Blur, and Visibility Controls](43-layout-blur-visibility.md)
- [44 — Filter and Discovery Experience](44-filter-discovery-experience.md)
- [45 — Required Packages and Dependencies](45-required-packages-dependencies.md)

## Implementation and handoff

- [46 — Implementation Plan](46-implementation-plan.md)
- [47 — Requirements Traceability Matrix](47-requirements-traceability.md)
- [48 — Open Questions and Decisions](48-open-questions-and-decisions.md)
- 49 — Documentation Index (this document)
- [50 — Project Handoff Checklist](50-project-handoff.md)

## Root documents (planned — not yet created)

This is a documentation-only planning package; no implementation exists yet, so none of the paths below are present in the repository. They are listed here as the intended locations once implementation begins.

- Project README (`../README.md`)
- Contributing (`../CONTRIBUTING.md`)
- Security policy (`../SECURITY.md`)
- Project status (`../PROJECT_STATUS.md`)
- Changelog (`../CHANGELOG.md`)

## Implementation assets (planned — not yet created)

- `assets/colors.json`
- `assets/colors.css`
- `assets/source-presets.json`
- `assets/taxonomy.json`
- `assets/ui-preferences.json`
- `assets/dependencies.json`
- `tui/CMakeLists.txt`
- `tui/CMakePresets.json`
- `tui/src/main.cpp`
- `scripts/install-tui-deps.sh`
