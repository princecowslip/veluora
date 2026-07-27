# Accessibility

## Target

Aim for WCAG 2.2 AA for the desktop GUI and equivalent keyboard and text accessibility for TUI and CLI surfaces.

## Keyboard access

Every GUI action must be reachable without a pointer.

Requirements:

- Visible focus indicator
- Logical focus order
- No keyboard traps
- Skip-to-content action
- Keyboard-accessible context menus
- Shortcut discovery
- Shortcut customization
- No single-key shortcuts while focus is in text input unless explicitly handled

## Screen readers

- All controls have accessible names.
- Media cards expose title, type, source, progress, and selected state.
- Decorative images are hidden.
- Thumbnail blur state is announced.
- Loading and source-completion changes use polite live regions.
- Errors and safety blocks use assertive announcements only when immediate action is required.
- Dialogs announce title and purpose.

## Player accessibility

- Captions and subtitles are supported.
- Subtitle track, language, and style are keyboard-accessible.
- Controls expose current value and range.
- Seek operations announce time.
- Volume is not changed on hover or scroll without focus.
- Autoplay is disabled by default.
- Animation can be paused.
- Reduced-motion preference is respected.

## Reader accessibility

Story reader:

- Adjustable font size and line spacing
- Reflow
- Text selection
- Search
- Screen-reader-friendly semantic structure
- Optional local text-to-speech
- Theme and contrast choices

Comic reader:

- Page number announcements
- Reading-order configuration
- Keyboard page navigation
- Optional page descriptions stored locally
- Zoom controls with state announcements

Automated image descriptions should be opt-in and local where practical due to privacy sensitivity.

## Color and contrast

- Text contrast meets AA.
- Focus indicators meet non-text contrast requirements.
- Progress, download, error, and source states include labels or icons.
- High-contrast mode avoids translucent overlays.
- Blur placeholders remain distinguishable from loading placeholders.

## Motion and flashing

- No flashing content is introduced by the application.
- Source-provided animated previews do not autoplay by default.
- Reduce-motion mode removes nonessential transitions.
- GIFs and animations include pause controls.
- Scrubbing previews can be disabled.

## Cognitive accessibility

- Use consistent terminology.
- Keep primary actions in stable locations.
- Explain technical failures in plain language.
- Break complex setup into steps.
- Show consequences before destructive actions.
- Avoid shame-based or moralizing language.
- Keep safety language clear and direct.

## Terminal accessibility

- Monochrome mode
- Text labels for all states
- Configurable key map
- No reliance on box-drawing characters for meaning
- Compatible plain-output mode
- Predictable screen updates
- Ability to disable inline image previews
