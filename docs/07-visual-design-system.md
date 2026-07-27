# Visual Design System

## Design direction

Use a restrained, cinematic interface with strong legibility and minimal visual noise. The system should avoid stereotypical adult-site aesthetics such as aggressive saturated accents, flashing banners, or dense advertising layouts.

The design should work equally well for private local libraries and source browsing.

## Theme strategy

Provide:

- Dark theme as the default
- Light theme
- System theme
- Extra-dim viewing theme
- High-contrast theme

A neutral theme should be available for shared-device use.

The dark theme's canvas is pure black. See [30 — Design Tokens and UI Colour Roles](30-design-tokens.md) for the concrete hex values and named accent roles (Indigo, Lavender, Iris, Violet, Moonstone, Mint, Seafoam, Aquamarine, Yellow, Red) that back the semantic tokens below; `accent.primary` maps to Indigo.

## Color roles

Use semantic tokens rather than hard-coded colors:

```text
surface.canvas
surface.panel
surface.elevated
surface.overlay

text.primary
text.secondary
text.muted
text.inverse

border.subtle
border.standard
border.strong

accent.primary
accent.hover
accent.pressed

status.success
status.warning
status.error
status.info

privacy.locked
privacy.private
safety.blocked
download.active
source.degraded
```

### Color requirements

- Primary text must meet WCAG AA contrast.
- Status must never be communicated by color alone.
- Blocked or prohibited states should use an icon and label.
- Source identity colors must not override application accessibility.
- Thumbnail overlays need a contrast scrim.

## Typography

Recommended roles:

- Display: page title and media title
- Heading: section title
- Body: descriptions and settings
- Label: metadata and controls
- Mono: commands, file paths, hashes, technical details
- Reader: long-form story content

Typography controls:

- Interface scale
- Reader font family
- Reader text size
- Reader line height
- Reader line width
- Dyslexia-friendly font option when available
- Monospace override for terminal-like views

## Spacing and density

Base spacing unit: 4 px.

Common values:

- 4: tight internal spacing
- 8: icon-to-label spacing
- 12: compact component padding
- 16: standard component padding
- 24: section spacing
- 32: page grouping
- 48: major separation

Density modes:

- Comfortable
- Compact
- Touch

## Shape and elevation

- Small controls: 6 px radius
- Cards and panels: 10 px radius
- Dialogs: 12 px radius
- Pills and tags: fully rounded
- Avoid excessive shadows in dark mode.
- Use border and surface contrast before elevation.

## Iconography

Icons should be simple, consistent, and recognizable at 16–24 px.

Required icons include:

- Play, pause, next, previous
- Image, animation, story, audio, comic
- Source, local file, cloud, cache, download
- Lock, private session, clear history
- Block, report, warning
- Favorite, collection, queue
- Search, filter, sort
- External link
- Plugin and permission
- Offline and expired

Icons need text labels in ambiguous or safety-critical contexts.

## Thumbnail system

Aspect-ratio presets:

- Poster: 2:3
- Landscape: 16:9
- Square: 1:1
- Page: original ratio constrained by height
- Story: generated cover or typography card
- Audio: cover art or waveform card

Thumbnail overlays may show:

- Media type
- Duration
- Page count
- Progress
- Download state
- Source badge
- Private blur state

Avoid more than three simultaneous overlays.

## Motion

Motion should be subtle and optional.

Allowed:

- Crossfade between viewer items
- Progress animation
- Panel slide
- Loading skeleton
- Download state transition

Avoid:

- Autoplaying previews by default
- Parallax
- Flashing elements
- Bouncing calls to action
- Long modal transitions

Respect reduced-motion preferences.

## Component inventory

- App shell
- Navigation rail
- Top bar
- Command palette
- Search field
- Filter chips
- Media card
- Result row
- Source badge
- Tag chip
- Progress bar
- Player controls
- Reader toolbar
- Metadata drawer
- Collection picker
- Download row
- Permission prompt
- Lock screen
- Privacy mode banner
- Empty state
- Error notice
- Toast
- Dialog
- Context menu
- Pagination and infinite-scroll sentinel
- TUI-equivalent state labels

## Content-safe presentation

The design system must support:

- Blurred thumbnails
- Neutral placeholders
- Hidden titles
- Media-type-only cards
- Locked metadata panels
- Source badges without explicit source names, when neutral mode is enabled
