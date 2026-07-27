# Layout, Blur, and Visibility Controls

## Machine-readable preferences

- `assets/ui-preferences.json` (planned — not yet created; generated from the layout rules below)

## Layout system

### Home feed layouts

- Comfortable cards
- Compact cards
- List
- Masonry
- Group by source
- Timeline

### Library layouts

- Poster grid
- Compact grid
- Masonry
- Justified image grid
- Detailed list
- Table
- Timeline
- Creator shelf
- Series shelf

### Discover layouts

- Unified grid
- Group by source
- Group by category
- Masonry
- List

### Detail layouts

- Right-side panel
- Full page
- Bottom sheet
- Hidden

### Viewer layouts

Image:

- Fit window
- Fit width
- Fit height
- Original size
- Filmstrip
- Slideshow

Story:

- Single column
- Wide column
- Two columns
- Paged
- Continuous

Manga and comics:

- Single page
- Double page
- Vertical strip
- Left-to-right
- Right-to-left

## Layout memory

Layout may be remembered:

- Globally
- Per view
- Per media type
- Per collection
- Per source
- Per series

Example:

- Images use justified grid.
- Video uses poster grid.
- Stories use detailed list.
- Manga opens right-to-left vertical strip.
- One collection uses manual table order.

## Blur styles

### None

No privacy effect.

### Soft blur

Approximately 8 px. Suitable for personal-server thumbnails.

### Medium blur

Approximately 16 px. Hides details but preserves colour and composition.

### Strong blur

Approximately 28 px. Default for newly enabled public sources.

### Pixelate

Low-resolution preview scaled up with nearest-neighbour rendering.

### Silhouette

Dominant-colour or gradient placeholder.

### Solid placeholder

Neutral media-type card with no preview.

## Reveal methods

- Always visible
- Reveal on hover
- Reveal on keyboard focus
- Reveal on click
- Reveal while pressing and holding
- Reveal after unlock
- Reveal for current session
- Never reveal thumbnail

Keyboard focus must not reveal sensitive thumbnails unless the profile explicitly enables it.

## Blur scopes

Rules can apply to:

- Global profile
- Source
- Category
- Tag
- Collection
- Saved search
- Exact item

Priority:

```text
policy block
→ exact item
→ user tag or category
→ source
→ collection or saved search
→ profile
→ global default
```

## Suggested defaults

### Local

- Blur: none
- Reveal: always
- Locked state: solid placeholder

### Personal server

- Blur: soft
- Reveal: after unlock
- Cache: limited

### Public source

- Blur: strong
- Reveal: click or session
- Cache: session-only
- Home feed: off

### Unknown rating

- Blur: solid
- Reveal: only after review

## Category controls

Users can choose for every category or orientation:

- Show normally
- Blur
- Hide from Home
- Hide from Discover
- Hide everywhere
- Require unlock
- Block permanently

## Card density

Comfortable:

- Larger thumbnail
- Description
- Tags
- Secondary actions

Compact:

- Smaller thumbnail
- Title
- Source
- Progress
- Overflow menu

Touch:

- Larger targets
- Fewer overlays
- Bottom-sheet actions

## Layout accessibility

- Preserve keyboard order in masonry layouts.
- Expose semantic list order to screen readers.
- Avoid visual-only grouping.
- Maintain focus when changing layout.
- Provide text alternatives for blurred cards.
- Do not reveal through screen-reader labels while locked.
