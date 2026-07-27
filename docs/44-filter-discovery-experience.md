# Filter and Discovery Experience

## Filter drawer

Sections:

1. Media
2. Categories
3. Sexual orientation
4. Participant composition
5. Acts and themes
6. People and series
7. Source
8. Technical
9. Progress and library state
10. Visibility and blur

## Quick filter chips

Examples:

```text
Video
Local
Unviewed
Portrait
Lesbian
Gay
Bisexual
Stories
Comics
Under 20 min
Downloaded
Blurred
```

Users choose which sensitive quick filters appear in the interface.

## Category browser

The category browser uses large neutral cards with icons rather than explicit thumbnails when locked.

Possible top-level cards:

```text
Solo
Couples
Group
Queer
Trans & nonbinary
Animation
Manga & comics
Stories
Audio
VR
Fetish & kink
Romance
```

## Orientation filter

Orientation may use:

- Multi-select chips
- Include and exclude
- Any or all logic
- Unspecified toggle
- Hide from Home
- Save as preference

Example:

```text
Include: Lesbian women, Bisexual
Exclude: Unspecified
Match: Any
```

## Tag explorer

Views:

- Alphabetical
- Popular locally
- Recently used
- By namespace
- By source
- Related tags
- Blocked tags
- User tags

## Filter persistence

Filters can be:

- Temporary
- Remembered for the view
- Saved as search
- Pinned to Home feed
- Converted to smart collection

## Result explanations

Each result may explain:

```text
Matches orientation:bisexual
Matches saved search “New comics”
From pinned source
Filtered locally because source lacks this field
```

## CLI examples

```bash
veloura search 'orientation:gay-men media:video'
veloura search 'orientation:lesbian-women composition:couple'
veloura search 'visual-orientation:portrait media:image'
veloura search 'category:manga-and-comics layout:vertical-strip'
veloura search 'act:bondage blur:strong'
veloura browse --source stash --category couples
```

## TUI filters

```text
[F1] Media
[F2] Categories
[F3] Orientation
[F4] Tags
[F5] Source
[F6] Visibility
```

Sensitive fields can be replaced with user-defined neutral labels in shared-device mode.
