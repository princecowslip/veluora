# Release Checklist

## Product

- [ ] Scope matches the published release notes.
- [ ] No incomplete feature appears as fully supported.
- [ ] Source capability limitations are visible.
- [ ] Privacy and safety documentation is current.

## Functional

- [ ] Local indexing passes.
- [ ] All supported media types open.
- [ ] Resume works.
- [ ] Search works across supported scopes.
- [ ] Favorites and collections persist.
- [ ] Downloads honor capability rules.
- [ ] Block rules apply everywhere.
- [ ] Import and export pass.

## Privacy

- [ ] Telemetry remains off by default.
- [ ] Default logs contain no sensitive titles, queries, tags, or paths.
- [ ] Private session leaves no configured history.
- [ ] Clear-history verification passes.
- [ ] Credentials can be removed.
- [ ] Locked mode hides thumbnails and titles.
- [ ] Diagnostic bundle is redacted.

## Security

- [ ] Dependency scan passes.
- [ ] Release is signed.
- [ ] Local API binds only as configured.
- [ ] Cross-origin protections pass.
- [ ] Archive traversal tests pass.
- [ ] Decoder containment tests pass.
- [ ] Plugin permissions are enforced.
- [ ] Connector revocation list is current.
- [ ] Database migration backup and rollback tested.

## Accessibility

- [ ] Keyboard-only flows pass.
- [ ] Focus indicators visible.
- [ ] Screen-reader labels reviewed.
- [ ] Contrast meets target.
- [ ] Reduced motion works.
- [ ] TUI monochrome mode works.
- [ ] CLI plain output works.

## Packaging

- [ ] Clean installation tested.
- [ ] Upgrade tested from previous supported version.
- [ ] Uninstall behavior documented.
- [ ] Configuration and data paths documented.
- [ ] Licenses included.
- [ ] External player detection tested.
- [ ] Automatic updater verifies signatures.

## Connectors

- [ ] Contract tests pass.
- [ ] Authentication tested.
- [ ] Rate-limit behavior tested.
- [ ] Deleted items handled.
- [ ] Expired media refreshed.
- [ ] Download permission represented correctly.
- [ ] Source terms and policy reviewed.

## Release communication

- [ ] Changelog
- [ ] Known issues
- [ ] Migration notes
- [ ] Privacy-impact changes
- [ ] Connector changes
- [ ] Security advisories
- [ ] Rollback instructions
