# Release Polish Checklist

## First-run

- [ ] Welcome copy is clear.
- [ ] Privacy profile choices are understandable.
- [ ] Local-only setup is fully supported.
- [ ] Source setup can be skipped.
- [ ] Playback test is optional.
- [ ] Setup can be resumed later.

## Navigation

- [ ] Back restores scroll and filters.
- [ ] Command palette covers major actions.
- [ ] Current location is clear.
- [ ] Empty states have useful actions.
- [ ] No critical action is hidden only in a context menu.

## Home

- [ ] Continue is accurate.
- [ ] Feed is user-controlled.
- [ ] Public source feed is opt-in.
- [ ] Feed cards explain why shown.
- [ ] Feed filters persist.
- [ ] Notices do not dominate the page.

## Library

- [ ] Filters are removable chips.
- [ ] Batch actions are safe.
- [ ] Duplicate grouping is understandable.
- [ ] Grid, list, and detail views remain consistent.
- [ ] Blocked items do not render thumbnails.
- [ ] Selected state is obvious.

## Viewer

- [ ] Resume works.
- [ ] Controls hide and return predictably.
- [ ] Esc behavior is consistent.
- [ ] Keyboard shortcuts do not trigger while typing.
- [ ] Error states preserve progress.
- [ ] External player failure has a recovery action.

## Sources

- [ ] Capabilities are clear.
- [ ] Permission summaries are readable.
- [ ] Login state is visible.
- [ ] Rate limiting is not shown as empty results.
- [ ] Download is hidden when not permitted.
- [ ] Remove source and remove credentials are separate.

## Privacy

- [ ] Start locked works.
- [ ] Neutral mode hides sensitive UI.
- [ ] Private session leaves no configured history.
- [ ] Clear-data operation reports success or failure.
- [ ] Default logs remain redacted.
- [ ] Notifications are neutral by default.

## Accessibility

- [ ] Keyboard-only use passes.
- [ ] Focus rings are visible.
- [ ] Contrast passes.
- [ ] Reduced motion passes.
- [ ] Screen-reader labels are complete.
- [ ] TUI monochrome mode passes.
- [ ] CLI plain output passes.

## Visual polish

- [ ] Pure-black canvas is consistent.
- [ ] Panels preserve depth without excessive shadows.
- [ ] Indigo remains the primary accent.
- [ ] Yellow is reserved for warning.
- [ ] Red is reserved for failure and destructive action.
- [ ] Seafoam and aquamarine identify playback and progress.
- [ ] No screen uses every accent simultaneously.
- [ ] Loading skeletons match final layout.
- [ ] Empty thumbnails have intentional placeholders.

## Copy polish

- [ ] Labels use direct verbs.
- [ ] Errors explain recovery.
- [ ] Technical terms are hidden in simple mode.
- [ ] Safety copy is clear and neutral.
- [ ] Destructive actions name the data being deleted.
- [ ] No shame-based language appears.

## Performance

- [ ] Warm start meets target.
- [ ] Search begins quickly.
- [ ] Indexing does not freeze browsing.
- [ ] Feed loads progressively.
- [ ] Viewer has priority over background work.
- [ ] Thumbnail queue uses bounded concurrency.
