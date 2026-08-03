# Content Safety and Compliance

## Scope

This document defines product controls, not legal advice. A public or hosted release requires specialist review in every supported jurisdiction.

## Implementation status

This document is a design target for the blocking/safety system. Today,
`domain::BlockRule` exists and is consulted in exactly one place —
`DownloadService`'s eligibility check — but there is no CLI command,
`local-api` route, or GUI/TUI screen to create, list, or remove a block
rule anywhere. See `KNOWN_ISSUES.md`'s "Content safety and blocking"
section.

## Prohibited material

The application must not knowingly index, retain, display, distribute, or facilitate access to:

- Child sexual abuse material
- Sexualized depictions of minors
- Material where participants' adult status cannot reasonably be established
- Non-consensual intimate imagery
- Hidden-camera intimate material
- Trafficking or exploitation
- Real sexual assault
- Content published without performer consent
- Doxxing or exposed personal information
- Malware presented as media
- Material prohibited by applicable law

## Product boundaries

The product must not:

- Bypass paywalls, DRM, age gates, authentication, or geographic controls.
- Circumvent source moderation.
- Rehost public copies without authorization.
- Remove provenance or source attribution.
- Provide automated performer identification.
- Provide tools for evading takedowns.
- Import unrestricted arbitrary websites.

## Source approval

Official connectors require review of:

- Source legality and reputation
- Terms of service
- API or feed authorization
- Age and consent policies
- Takedown process
- Authentication model
- Download rights
- Rate limits
- Data retention
- Jurisdictional restrictions

## Safety statuses

The shipped `domain::media_item::SafetyStatus` enum is a simpler 4-value
model:

```text
Unreviewed
Approved
Flagged
Blocked
```

The richer 7-value model below (`Unknown`/`Allowed`/`UserBlocked`/
`SourceBlocked`/`PolicyBlocked`/`ReviewRequired`/`Removed`) remains a
design target, not what's implemented:

```text
Unknown
Allowed
UserBlocked
SourceBlocked
PolicyBlocked
ReviewRequired
Removed
```

Policy-blocked items should not render thumbnails or descriptions.

## User controls (design target — no UI exists yet)

- Block source
- Block creator
- Block series
- Block tag
- Block exact item
- Block file or perceptual hash
- Never show again
- Review blocked items without media preview
- Export and import block rules

## Reporting

For official connectors, users should be able to report:

- Suspected prohibited content
- Non-consensual content
- Incorrect age classification
- Malware
- Broken source attribution
- Connector bypassing source rules

Reports should avoid uploading local files automatically. Ask the user what data will be shared.

## Takedown and deletion

For hosted modes:

- Publish a reporting channel.
- Preserve minimum audit evidence.
- Remove access promptly when required.
- Respect source deletion and consent withdrawal.
- Prevent re-import through canonical IDs or hashes where appropriate.
- Document appeal and correction processes.

## Age assurance

A local application may use adult-use confirmation and optional device lock.

A hosted service may require:

- Jurisdiction-specific age assurance
- Data minimization
- separation between age verification and browsing activity
- retention limits
- regional restrictions
- accessibility accommodations

## Connector governance

A connector may be disabled when:

- The source becomes compromised.
- The source no longer permits API use.
- The connector begins bypassing access controls.
- Safety or consent practices become unacceptable.
- Legal risk changes.
- The connector distributes malware.

## Synthetic test content

Testing should use:

- Abstract generated thumbnails
- Public-domain media
- non-explicit synthetic fixtures
- generated metadata
- deliberately malformed files

Do not use real prohibited content in tests.
