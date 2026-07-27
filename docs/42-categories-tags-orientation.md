# Categories, Tags, and Orientation Taxonomy

## Purpose

Veloura needs one normalized taxonomy without erasing the original source tags.

The machine-readable taxonomy will be stored in:

- `assets/taxonomy.json` (planned — not yet created; generated from the taxonomy defined below)

## Important distinction

Veloura stores two different concepts:

### Visual orientation

The shape of the media:

- Landscape
- Portrait
- Square
- Panorama
- Vertical video
- Unknown

### Sexual orientation category

A content classification used for browsing:

- Heterosexual
- Gay men
- Lesbian women
- Bisexual
- Pansexual
- Queer
- Mixed or multi-orientation
- Solo or not applicable
- Unspecified

A scene or story category must never be used to infer a real performer’s personal sexual orientation.

## Gender identity

Gender identity is separate from sexual orientation.

Supported normalized values may include:

- Woman
- Man
- Nonbinary
- Trans woman
- Trans man
- Intersex
- Mixed
- Unspecified

For real people, identity metadata must come from self-description or a trusted source record. It must not be inferred from appearance.

## Top-level categories

Suggested user-facing categories:

- Solo
- Couples
- Group
- Queer
- Trans and nonbinary
- Amateur
- Professional
- Animation
- Illustration
- Manga and comics
- Stories
- Audio
- VR
- Interactive
- Fetish and kink
- Romance
- Instructional
- Compilation

Categories are broad navigation labels. Tags provide detail.

## Tag namespaces

### Identity and composition

```text
orientation:
gender:
composition:
relationship:
```

Examples:

```text
orientation:lesbian-women
composition:couple
gender:mixed
relationship:partners
```

### Media and structure

```text
media:
format:
layout:
visual-orientation:
series:
chapter:
gallery:
```

### Content

```text
act:
genre:
theme:
setting:
production:
```

Examples include:

- Masturbation
- Mutual masturbation
- Oral
- Vaginal
- Anal
- Toys
- Massage
- Striptease
- Dance
- Bondage
- Dominance and submission
- Roleplay
- Group sex
- Non-penetrative

These tags are optional filters. They should not be required for local indexing.

### People and attribution

```text
creator:
performer:
studio:
artist:
author:
narrator:
```

### Technical

```text
resolution:
duration:
pages:
language:
subtitle:
audio:
codec:
```

### User-owned

```text
user:
collection:
rating:
note:
```

User tags remain local and are never sent to a source.

### Safety and visibility

```text
safety:
visibility:
blur:
blocked:
```

## Tag display modes

- Hidden
- Compact
- Grouped
- Full
- Original source tags
- Normalized tags
- Both original and normalized

Grouped display should use sections such as:

```text
Orientation
Composition
Genre
Acts
People
Series
Source
Technical
User tags
```

## Aliases and mappings

Example:

```text
source term → normalized term
f/f         → orientation:lesbian-women
m/m         → orientation:gay-men
straight    → orientation:heterosexual
bi          → orientation:bisexual
portrait    → visual-orientation:portrait
```

Mappings must be source-specific and reviewable.

## Search examples

```text
orientation:bisexual media:video
orientation:gay-men composition:couple
category:stories genre:romantic
visual-orientation:portrait media:image
act:bondage -tag:user:block
source:local layout:vertical-strip
```

## Autocomplete

Autocomplete should:

- Group terms by namespace.
- Show normalized and source terms.
- Hide policy-blocked terms.
- Show which sources support the term.
- Explain whether a filter is applied remotely or locally.
- Respect blocked namespaces.

## Privacy

Users may disable or hide orientation, gender, or act metadata entirely.

Options:

- Do not display
- Display only in details
- Use as filters but not badges
- Hide from Home
- Hide from notifications
- Exclude from search history
