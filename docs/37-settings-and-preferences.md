# Settings and Preferences

## Implementation status

This document is a design target cataloguing roughly 90 settings across 11
categories. The real GUI Settings screen
(`crates/gui/src/screens/settings.rs`) implements about 8: theme
(Dark/Light), library folders, external player path, password/
start-locked, metadata encryption, backup/restore, and diagnostics export.
Some backend equivalents for other sections exist without Settings-screen
UI (e.g. download quota via `PrivacyService`/`DownloadService`), but most
of the catalogue below is unbuilt.

## General

- Launch at startup
- Start locked
- Start on Home, Library, or Continue
- Simple or advanced mode
- Language
- Region
- Interface scale
- Check for updates

## Appearance

- Dark
- Light
- System
- Pure-black OLED
- High contrast
- Accent intensity
- Thumbnail radius
- Compact or comfortable density
- Reduce motion
- Neutral mode

## Home

- Reorder sections
- Show Continue
- Show Feed
- Show Recently Added
- Show Pinned Collections
- Show Source Status
- Feed density
- Feed filters
- Recommendations
- Seen-item retention

## Library

- Default view
- Default sort
- Card size
- Show source badges
- Show progress
- Collapse duplicates
- Remember filters
- Open details in panel or page
- Folder scan schedule

## Playback

- Default player
- External player command
- Autoplay next
- Remember speed
- Completion threshold
- Resume prompt
- Subtitle defaults
- Audio language
- Hardware decoding
- Picture-in-picture
- Lock-screen behavior

## Reader

- Font
- Text size
- Line height
- Line width
- Theme
- Reading direction
- Double-page mode
- Margin crop
- Page preload count
- Text-to-speech provider

## Sources

Per source:

- Enabled
- Unified search
- Home feed
- Authentication
- Cache
- Downloads
- Thumbnails
- Rating mapping
- History
- Request limit
- Open externally
- Permissions

## Downloads

- Default folder
- Naming template
- Maximum concurrent downloads
- Verify checksum
- Keep partial files
- Cache quota
- Permanent-download quota
- Low-storage threshold
- Automatic cleanup

## Privacy

- Auto-lock timeout
- Private-session default
- History retention
- Search retention
- Thumbnail retention
- Neutral notifications
- Hide titles
- Blur external thumbnails
- Encrypted metadata
- Panic shortcut
- Clear data
- Export data

## Safety

- Allowed ratings
- Unknown-rating behavior
- Blocked sources
- Blocked tags
- Blocked creators
- Blocked series
- Exact-item blocks
- Hash exclusions
- Policy status

## Accessibility

- High contrast
- Screen-reader optimizations
- Reduced motion
- Monochrome TUI
- Large controls
- Custom shortcuts
- Caption preferences
- Reader typography
- Focus indicator strength

## Advanced

- Local API
- Connector diagnostics
- Plugin permissions
- External command templates
- Database maintenance
- Thumbnail regeneration
- Search-index rebuild
- Support bundle
- Developer mode
