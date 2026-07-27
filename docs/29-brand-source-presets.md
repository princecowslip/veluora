# Brand, Source Presets, and Aesthetic

## Product name

# Veloura

**Tagline:** A private library for every format.

### Why it works

- **Veloura** blends the ideas of velvet, aura, and visual luxury.
- It feels refined, cinematic, and discreet without sounding clinical.
- The name works for video, images, stories, audio, manga, comics, and personal libraries.
- It is suitable for desktop, terminal, and command-line branding:
  - Veloura Desktop
  - Veloura TUI
  - Veloura CLI
- The launcher and icon remain neutral enough for shared-device use.
- The name supports the Midnight Gallery aesthetic and the indigo-to-lavender palette.

### Technical naming

```text
Product: Veloura
Short name: Veloura
CLI command: veloura
Internal package prefix: veloura_
URI scheme: veloura://
Configuration directory: veloura/
TUI mark: V/
```

Before public release, perform formal trademark, package-name, application-store, social-handle, and domain clearance.

## Naming decision

The final one-word product name is **Veloura**. The wordmark, app title, TUI header, package metadata, and documentation should use this form consistently. Formal trademark, package-name, application-store, social-handle, and domain clearance is still required before public release.

## Alternative names

| Name | Character | Notes |
|---|---|---|
| Velvet Atlas | Warm and premium | More sensual, less discreet |
| Veilmark | Private and technical | Strong privacy-tool character |
| Umbra Library | Clear and atmospheric | Slightly more conventional |
| Nocturne Index | Elegant and editorial | Less distinctive as a single word |
| Afterdark Archive | Direct and memorable | Not suitable for neutral mode |

## Brand personality

Veloura should feel:

- Private
- Assured
- Editorial
- Calm
- Media-focused
- Technically capable
- Non-judgmental
- Deliberate rather than provocative

It should not feel:

- Cheap
- Loud
- Neon-saturated
- Casino-like
- Advertising-heavy
- Social-media-driven
- Porn-site-derived
- Overly corporate
- Shameful or secretive

## Aesthetic direction

### Midnight Gallery

The visual concept combines:

- A private screening room
- An editorial image archive
- A high-end media player
- A quiet digital library

The application uses a pure-black canvas, warm off-white typography, an indigo interaction color, and restrained seafoam and aquamarine playback/progress accents. Media remains visually dominant while controls recede. See [30 — Design Tokens and UI Colour Roles](30-design-tokens.md) for the full accent-role system and hex values, which this brand direction follows exactly.

The design should avoid generic cyberpunk styling. Neon gradients are reserved for branding, selected states, and brief progress accents rather than being applied to every component.

## Core palette

### Dark theme

| Token | Hex | Use |
|---|---|---|
| Canvas | `#000000` | Main application background |
| Surface | `#11151D` | Navigation and standard panels |
| Elevated | `#181E29` | Dialogs, menus, selected cards |
| Raised | `#202735` | Hovered and active surfaces |
| Border subtle | `#252C39` | Card and panel separation |
| Border strong | `#394355` | Focused or emphasized boundaries |
| Text primary | `#F2F0EA` | Titles and important text |
| Text secondary | `#AAB1BE` | Body copy and metadata |
| Text muted | `#747D8C` | Hints and inactive metadata |
| Accent indigo | `#6366F1` | Primary buttons and selected controls |
| Accent violet | `#8B5CF6` | Active emphasis and favorite state |
| Accent lavender | `#C4B5FD` | Soft selection surfaces, collections, and bookmarks |
| Playback seafoam | `#2DD4BF` | Playback, active stream, completed download |
| Progress aquamarine | `#22D3EE` | Download and discovery progress |
| Success mint | `#34D399` | Successful operations |
| Warning yellow | `#FFD166` | Rate limits and recoverable warnings |
| Danger red | `#EF4444` | Destructive actions and errors |
| Info moonstone | `#94A3B8` | Lock, encryption, source identity, and private-session state |
| Focus | `#A5B4FC` | Keyboard focus ring |

These match the accent roles defined in [30 — Design Tokens and UI Colour Roles](30-design-tokens.md); this table adds the surface, text, and border tones needed for a full theme.

### Brand gradient

```text
#6366F1 → #8B5CF6 → #C4B5FD
```

Use only for:

- App icon
- Logo accent
- Onboarding progress
- Active playback scrubber
- Selected command-palette item
- Marketing artwork

Do not use the gradient behind long text or across large application surfaces.

### Light theme

| Token | Hex | Use |
|---|---|---|
| Canvas | `#F3F1EC` | Main background |
| Surface | `#FFFFFF` | Standard panels |
| Elevated | `#F8F7F3` | Dialogs and cards |
| Border | `#D7D9DF` | Dividers |
| Text primary | `#191B22` | Titles |
| Text secondary | `#555D6A` | Body copy |
| Accent indigo | `#4F46E5` | Primary actions |
| Accent violet | `#7C3AED` | Favorite and brand highlights |
| Info moonstone | `#64748B` | Lock and privacy state |

All combinations must be contrast-tested before implementation. Semantic status must also include an icon or text label.

## Type system

### Interface font

Use a highly legible modern sans-serif with:

- Open counters
- Clear punctuation
- Strong numeral differentiation
- Broad language support
- Variable font support where possible

Suggested categories:

- Humanist sans for interface text
- Editorial serif as an optional story-reader font
- Monospace for CLI, hashes, paths, and technical metadata

### Type scale

```text
Display       32/38, semibold
Page title    24/30, semibold
Section       18/24, semibold
Body          15/22, regular
Metadata      13/18, regular
Label         12/16, medium
Micro         11/14, medium
```

Avoid all-uppercase text except short badges.

## Logo and icon concept

### Primary logo

A geometric `S` made from two offset media cards:

- Upper card includes a subtle play-triangle cutout.
- Lower card includes a page or index-tab cutout.
- Negative space forms an `S`.
- The mark works in one color.
- The full-color version uses the indigo-to-lavender gradient.

### App icon

- Pure-black rounded square
- Centered Veloura mark
- Thin off-white edge highlight
- No explicit imagery
- Neutral-mode version uses monochrome graphite and silver

### Favicon and TUI mark

```text
V/
```

The slash suggests an index path, command syntax, and forward navigation.

## Surface and component style

### Cards

- 10 px radius
- Thin border
- Minimal shadow
- Thumbnail fills upper region
- Metadata remains outside the image where possible
- Selected state uses indigo border and a soft inner glow

### Buttons

Primary:

- Indigo fill
- Off-white text
- No gradient except special onboarding actions

Secondary:

- Raised graphite surface
- Standard border
- Primary text

Danger:

- Transparent or dark surface by default
- Red emphasis only after focus or confirmation

### Tags

Tag families may use subtle tinted surfaces:

- Creator: violet
- Series: moonstone
- Source: graphite
- Language: seafoam
- User tag: lavender
- Blocked: red
- Technical: neutral grey

Keep text labels visible; color is supplementary.

## Motion

Motion language:

- 120–180 ms for small controls
- 180–240 ms for panels
- Soft deceleration
- Crossfade for media changes
- No bouncing
- No autoplay previews by default
- Respect reduced-motion preferences

## Source preset philosophy

Presets are connector templates, not endorsements.

Every preset must be:

- Disabled unless it is a local source.
- Explicitly enabled by the user.
- Limited to a declared API, feed, protocol, or local server.
- Subject to source terms, rate limits, authentication, and regional rules.
- Independently removable.
- Unable to bypass access controls.
- Unable to download unless the connector declares download permission.

## Preset source groups

### 1. Local sources

Enabled during onboarding only after the user selects them.

| Preset | Default | Media | Capabilities |
|---|---|---|---|
| Local folders | Setup choice | All supported local formats | Index, search, open, organize |
| Managed library | Off | All supported formats | Import, organize, verify |
| Removable drive | Off | All supported formats | Temporary or persistent index |
| Watch folder | Off | All supported formats | Automatic import |
| Local playlist | Off | Video and audio | Queue and playback |
| Local OPDS fixture | Off | Stories, manga, comics | Browse and read |

### 2. Personal media servers

All are off by default and require a server address and credentials.

| Preset | Best for | Suggested integration |
|---|---|---|
| Jellyfin | Video, audio, and images | Server API and authorized streaming |
| Kavita | Manga, comics, books, and EPUB | API, OPDS, or OPDS-PS |
| Komga | Comics, manga, magazines, and ebooks | API and OPDS |
| Audiobookshelf | Audio stories, audiobooks, podcasts, and basic ebooks | Server API |
| Generic OPDS | Stories, books, manga, and comics | OPDS 1 or 2 |
| Generic WebDAV | User-owned files | Authenticated file browsing |
| Generic S3-compatible storage | User-owned media | Read-only object listing |

For remote personal servers, require HTTPS unless the address is explicitly recognized as local.

### 3. Booru and image-board API presets

All are off by default.

| Preset | Connector mode | Initial capability |
|---|---|---|
| Danbooru | Site-specific API | Search, browse, tags, pools, details |
| Gelbooru | Site-specific DAPI | Search, browse, tags, details |
| e621/e926 | Site-specific API | Search, browse, tags, pools, details |
| Danbooru-compatible | User-defined endpoint | Capability probe |
| Gelbooru-compatible | User-defined endpoint | Capability probe |
| Shimmie-compatible | User-defined endpoint | Read-only where documented |
| Custom booru JSON | Developer mode | User-supplied schema mapping |

Recommended restrictions:

- Read-only for the initial release.
- No voting, commenting, favoriting, or uploading.
- Metadata and thumbnail caching only.
- Original source attribution always visible.
- Explicit source rating controls.
- Per-source tag block rules.
- Conservative request limits.
- No automatic credential import.

### 4. Feeds and catalogues

| Preset | Default | Media |
|---|---|---|
| RSS | Off | Stories, audio, image posts, updates |
| Atom | Off | Stories, image posts, updates |
| JSON Feed | Off | Mixed media |
| OPDS | Off | Stories, ebooks, comics, manga |
| M3U/M3U8 playlist | Off | User-authorized video or audio |
| Local HTML import | Off | Saved stories and galleries |
| EPUB import | Off | Stories and comics |
| CBZ/CBR import | Off | Manga and comics |

Feeds should be treated as discovery metadata unless they directly provide authorized media files.

### 5. Browser handoff presets

For sites without a stable, documented, authorized API:

- Store only a user-created bookmark or search template.
- Open results in the system browser.
- Do not scrape pages.
- Do not import browser cookies.
- Do not resolve hidden media URLs.
- Do not offer downloads.
- Make the boundary visible with an `External` badge.

This allows familiar sites to appear in the source list without creating a fragile or non-compliant connector.

## Toggle design

Each source card contains one master toggle and expandable controls.

### Source card

```text
[icon] Gelbooru
       Booru API · Images · Animated images

       [ Enabled                          ○ ]
       Status: Not configured
       Authentication: Optional

       [Configure] [Test] [Permissions]
```

### Expanded toggles

```text
Enabled
Include in unified search
Show source feed on Home
Show thumbnails
Blur thumbnails initially
Cache metadata
Cache thumbnails
Allow permanent downloads
Use in private sessions
Retain search history
Open original source externally
```

`Allow permanent downloads` remains unavailable unless the connector reports that downloads are permitted.

## Recommended preset defaults

### Local source

```text
Enabled: user choice
Unified search: on
Home feed: on
Thumbnails: on
Blur initially: shared-device profile only
Metadata cache: on
Thumbnail cache: on
Downloads: not applicable
Private sessions: on
History: profile default
```

### Personal server

```text
Enabled: off
Unified search: on after configuration
Home feed: off
Thumbnails: on
Blur initially: on
Metadata cache: on
Thumbnail cache: limited
Downloads: off until explicitly enabled
Private sessions: on
History: profile default
```

### Public adult source

```text
Enabled: off
Unified search: off until enabled
Home feed: off
Thumbnails: blurred
Metadata cache: session-only
Thumbnail cache: session-only
Downloads: off
Private sessions: on
History: off
External source link: on
```

## Source status labels

Use plain-language states:

- Ready
- Disabled
- Setup required
- Authentication required
- Rate limited
- Temporarily unavailable
- Connector update required
- Blocked by policy
- Removed
- Offline

## Rating and visibility controls

Each compatible source should expose:

```text
Allowed ratings:
[ ] General
[ ] Suggestive
[ ] Explicit

Unknown rating:
( ) Hide
( ) Blur
( ) Show
```

Rating systems vary by source, so mappings must be shown and reviewable.

## Recommended Home layout

```text
Continue
Queue
Recently added locally
Pinned collections
Saved searches
Source status

External discovery feeds remain hidden until the user pins them.
```

This prevents the home screen from becoming a noisy multi-site feed and protects shared-device privacy.

## Brand copy examples

### Onboarding

> Your media stays yours. Add local folders or connect only the sources you choose.

### Empty Discover screen

> Enable a source to search beyond your local library.

### Private session

> Private session is active. Searches, viewing history, and temporary previews will be cleared when you exit.

### Source warning

> This source opens results externally. Veloura will not scrape, cache, or download its media.

### Download unavailable

> This source permits playback but does not declare download access.

## Final recommendation

Use **Veloura** with the **Midnight Gallery** aesthetic.

The identity is discreet enough for a privacy-focused local application, distinctive enough to brand, and flexible enough to cover every supported media format without making the interface resemble a conventional adult website.
