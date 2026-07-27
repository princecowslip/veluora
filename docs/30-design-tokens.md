# Design Tokens and UI Colour Roles

## Purpose

This document converts the Veloura visual system into implementation-ready tokens for desktop, webview, terminal, and documentation surfaces.

This document is the single source of truth for color values. A generated `assets/colors.json` and `assets/colors.css` are planned build outputs derived from the table below (not yet created); do not hand-author them separately from this file.

## Core rule

The canvas remains pure black. Accent colours are used as functional signals rather than decoration.

One component should normally have:

- One dominant accent
- One semantic status colour
- Neutral text and borders

Do not combine indigo, violet, seafoam, yellow, and red inside the same small card unless the colours represent genuinely different states.

## Accent roles

| Colour | Token | Hex | Primary role |
|---|---|---|---|
| Indigo | `accent.indigo` | `#6366F1` | Main action and navigation selection |
| Lavender | `accent.lavender` | `#C4B5FD` | Soft selection surfaces and glow |
| Iris | `accent.iris` | `#818CF8` | Secondary action and command emphasis |
| Violet | `accent.violet` | `#8B5CF6` | Active emphasis and favourites |
| Moonstone | `accent.moonstone` | `#94A3B8` | Information and source identity |
| Mint | `accent.mint` | `#34D399` | Success, ready, local availability |
| Seafoam | `accent.seafoam` | `#2DD4BF` | Playback and active media |
| Aquamarine | `accent.aquamarine` | `#22D3EE` | Progress, download and discovery |
| Yellow | `accent.yellow` | `#FFD166` | Warning, pending and attention |
| Red | `accent.red` | `#EF4444` | Error, blocked and destructive action |

## Base surfaces

| Role | Token | Hex |
|---|---|---|
| Canvas (dark, default) | `surface.canvas` | `#000000` |
| Text primary | `text.primary` | `#F2F0EA` |

The Indigo, Violet, and Lavender values, and the pure-black/off-white pairing, match the hex gradient already established in [33 — Veloura Logo Concept Sheet](33-logo-concept-sheet.md), so the brand mark and the UI accent system use identical color values.

## Red and yellow usage

### Red

Use red for:

- Delete confirmations
- Policy-blocked source state
- Permanent failure
- Invalid credential state after submission
- Unsafe plugin permission escalation
- Critical storage corruption
- Stop or cancel destructive process

Do not use red for ordinary selected states, favourites, or routine source offline states.

### Yellow

Use yellow for:

- Rate limiting
- Pending review
- Near-quota storage
- Authentication expiry approaching
- Partial search result
- Source capability mismatch
- Download waiting for user action

Yellow must include an icon or label because colour alone is insufficient.

## Component mapping

### Primary button

```css
background: var(--veloura-indigo);
color: var(--veloura-text-primary);
```

Hover uses iris. Pressed state darkens the surface rather than shifting to red or yellow.

### Selected navigation

- Indigo icon
- Lavender text or indicator
- Indigo soft background
- 2 px left or bottom indicator

### Playback

- Seafoam play state
- Aquamarine progress
- Moonstone buffered range
- Yellow temporary interruption
- Red unrecoverable playback error

### Source card

- Moonstone source badge
- Mint Ready
- Yellow Rate limited
- Red Blocked by policy
- Lavender selected source outline
- Indigo configuration action

### Download row

- Aquamarine active progress
- Mint complete
- Yellow paused or awaiting input
- Red failed
- Muted grey cancelled

## Terminal mapping

True-colour terminals should use the same hex values. Limited terminals map to:

```text
Indigo / Iris / Violet → Magenta or bright blue
Moonstone / Seafoam / Aquamarine → Cyan
Mint → Green
Yellow → Yellow
Red → Bright red
Lavender → Bright magenta or white
```

The TUI must always include text states such as `[WARN]`, `[BLOCKED]`, or `[DONE]`.
