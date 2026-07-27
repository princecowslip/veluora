# Feedback, Errors, and Notifications

## Feedback principles

The interface should always show:

- What is happening
- Whether the user can continue
- Whether data is safe
- What action is available

## Toasts

Use for brief reversible feedback:

- Added to collection
- Removed from queue
- Favourite saved
- Download queued
- Feed card hidden
- Filter cleared

Toasts may include Undo.

## Banners

Use for view-level conditions:

- Offline mode
- Source rate limited
- Partial search
- Storage near quota
- Private session active
- Connector update required

## Dialogs

Use for decisions:

- Delete local file
- Clear all history
- Remove source credentials
- Enable remote access
- Install high-permission plugin
- Merge duplicates
- Reset database

## Inline errors

Use near the affected control:

- Invalid folder
- Invalid source URL
- Login failed
- Unsupported query
- Unwritable download directory
- Invalid naming template

## Status colours

- Indigo: primary action
- Mint: success
- Moonstone: information
- Seafoam: active playback
- Aquamarine: progress
- Yellow: warning or pending
- Red: failure, block, or destructive action

Status always includes text or icon.

## Notification policy

Default:

- No explicit titles on operating-system notifications
- Download completion uses neutral wording
- Authentication warnings are allowed
- Critical storage warnings are allowed
- New-content notifications are off
- Public-source notifications are off

Examples:

> Veloura completed a download.

> A connected source needs attention.

> Storage is nearly full.

## Notification settings

Per event:

- Off
- In-app only
- Operating-system notification
- Notification with title
- Neutral notification

Events:

- Download complete
- Download failed
- New chapter
- Saved-search match
- Source authentication
- Source failure
- Storage warning
- Index complete
- Plugin disabled
