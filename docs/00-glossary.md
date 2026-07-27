# Glossary

**Connector:** A component that integrates one approved source or API family.

**Source:** A configured instance of a connector, including its settings and credentials.

**Media item:** A logical work that may have multiple source references or file variants.

**Variant:** A specific playable, readable, viewable, cached, or downloaded representation.

**Local override:** User-edited metadata that takes precedence over imported metadata without altering the source.

**Unified search:** One query executed across multiple enabled sources.

**Private session:** A session that minimizes or disables persistent history and temporary artifacts.

**Permanent download:** A user-requested file retained until the user deletes it.

**Cache:** Regenerable or expiring data managed automatically under a quota.

**Capability:** A connector-declared operation or query feature.

**Policy block:** An application-enforced block that cannot be bypassed through ordinary user settings.

**Simple mode:** The default interface mode for new users. Shows core navigation, essential filters, and guided source setup; hides raw connector capabilities, advanced query syntax, and technical/developer detail. See [34 — Features and Functions](34-features-and-functions.md).

**Advanced mode:** An optional interface mode that adds the full query language, connector diagnostics, plugin controls, and technical media-variant detail on top of Simple mode. See [34 — Features and Functions](34-features-and-functions.md).

**Neutral mode:** A presentation mode that replaces explicit source names, imagery, and branding with generic labels and a neutral icon/color treatment, for shared-device or discreet use. Distinct from a Private session, which governs history and temporary artifacts rather than on-screen labeling.

**Disabled (source or connector):** Turned off by configuration; can be re-enabled by the user or an administrator without losing its stored settings or credentials.

**Revoked (source or connector):** Terminated due to a policy, terms-of-service, or authentication failure outside the user's direct control; typically requires reconfiguration or is no longer offerable. See [23 — Operations and Observability](23-operations-and-observability.md) for the internal source-health enum and [29 — Brand, Source Presets, and Aesthetic](29-brand-source-presets.md) for the corresponding user-facing status labels.

**Visual orientation, sexual orientation categories, participant composition, gender identity categories, act tags, categories:** Optional classification fields on a Media item (see [13 — Domain and Data Model](13-data-model.md)) sourced from source metadata, local mapping, or explicit user edits. These describe declared or self-identified content attributes only; real-person identity or orientation must never be inferred from appearance or scene participation.
