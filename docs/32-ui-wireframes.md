# UI/UX Wireframes

These wireframes are structural references, not final visual designs. [51 — Sample Mock UI Specification](51-sample-mock-ui-spec.md) and [52 — Sample UI Specification](52-sample-ui-spec.md) describe the same Home/Library/Viewer screens at other levels of detail (mock states and interactions, and exact pixel measurements, respectively) — the three documents are complementary, not contradictory.

## Home

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ V/  VELOURA   [ Search library and sources…                 ] [Private] │
├───────────────┬─────────────────────────────────────────────────────────────┤
│ Home          │ HOME                                      [Feed settings]   │
│ Library       │                                                             │
│ Discover      │ Continue                                                    │
│ Collections   │ ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│ Downloads     │ │ Media card │ │ Media card │ │ Media card │               │
│ Settings      │ └────────────┘ └────────────┘ └────────────┘               │
│               │                                                             │
│ Sources       │ Feed   [All] [Local] [Sources] [Chapters] [Downloads]       │
│ ● Local       │ ┌─────────────────────────────────────────────────────────┐ │
│ ● Jellyfin    │ │ Thumbnail | New chapter / media update                 │ │
│ ! Source      │ │ Source · time · type             [Open] [•••]          │ │
│               │ └─────────────────────────────────────────────────────────┘ │
│               │ ┌─────────────────────────────────────────────────────────┐ │
│               │ │ Download complete                             [Reveal]  │ │
│               │ └─────────────────────────────────────────────────────────┘ │
│               │                                                             │
│               │ Recently added                                              │
│               │ [card] [card] [card] [card] [card]                         │
├───────────────┴─────────────────────────────────────────────────────────────┤
│ ▶ Item title               12:44 / 38:20          [Queue] [Device] [•••]   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Library

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ V/  LIBRARY      [ Search library… ] [Filters] [Sort] [Grid] [Select]      │
├──────────────┬──────────────────────────────────────────────┬───────────────┤
│ Media type   │ Results                                      │ Details       │
│ □ Video      │ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │ Preview       │
│ □ Images     │ │ card   │ │ card   │ │ card   │ │ card   │ │               │
│ □ Stories    │ │ 42%    │ │ local  │ │ warn   │ │       │ │ Title         │
│ □ Audio      │ └────────┘ └────────┘ └────────┘ └────────┘ │ Source        │
│ □ Comics     │ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │ Progress      │
│              │ │ card   │ │ card   │ │ card   │ │ card   │ │               │
│ Source       │ └────────┘ └────────┘ └────────┘ └────────┘ │ [Resume]      │
│ □ Local      │                                              │ [Favourite]   │
│ □ Server     │                                              │ [Collection]  │
│ □ External   │                                              │               │
│              │                                              │ Tags          │
│ Viewed       │                                              │ Description   │
│ Duration     │                                              │ Variants      │
└──────────────┴──────────────────────────────────────────────┴───────────────┘
```

## Video viewer

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ ← Library                   Item title                         [Info] [Queue]│
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                                                                             │
│                           PURE BLACK MEDIA STAGE                            │
│                                                                             │
│                                                                             │
│                                                                             │
│  ▶  12:44 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 38:20  🔊  CC  HD  ⛶    │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Story viewer

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ ← Library       Story title       Chapter 4 of 18       [Aa] [Search] [•••]│
├────────────────┬─────────────────────────────────────────────┬──────────────┤
│ Chapters       │                                             │ Details      │
│ 1              │              READING COLUMN                 │ Source       │
│ 2              │                                             │ Tags         │
│ 3              │  Paragraph text with adjustable width,      │ Progress     │
│ 4  ← current   │  size, spacing, theme and typography.       │ Notes        │
│ 5              │                                             │              │
│ …              │                                             │              │
├────────────────┴─────────────────────────────────────────────┴──────────────┤
│ Page/section progress ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Mobile or narrow layout

```text
┌──────────────────────────────┐
│ V/  Home       [Search] [Lock]│
├──────────────────────────────┤
│ Continue                     │
│ [ horizontal cards → ]       │
│                              │
│ Feed                         │
│ [All] [Local] [Sources]      │
│ ┌──────────────────────────┐ │
│ │ feed card                │ │
│ └──────────────────────────┘ │
│ ┌──────────────────────────┐ │
│ │ feed card                │ │
│ └──────────────────────────┘ │
├──────────────────────────────┤
│ Home Library Discover More  │
└──────────────────────────────┘
```

## Behaviour annotations

1. Feed cards load progressively.
2. External thumbnails remain blurred according to profile settings.
3. Yellow states are warnings or partial availability.
4. Red states are destructive, blocked, or unrecoverable.
5. Indigo is the dominant action colour.
6. Seafoam and aquamarine are reserved for active playback and progress.
7. Home feed is user-controlled and never auto-enables a public source.
