# Home Feed and Personalization

## Purpose

The Home feed helps users resume, discover updates, and understand system activity without becoming a noisy or uncontrolled public-content feed.

## Feed priorities

Default ranking:

1. Resume items
2. Queue items
3. Local additions
4. Followed series updates
5. Personal-server additions
6. Saved-search matches
7. Pinned public-source updates
8. Download completion
9. Source warnings
10. Connector update notices

## Feed eligibility

A feed item may appear when:

- It is local.
- It is from a connected personal server.
- The user follows the series or creator.
- The user pins the source.
- The user pins a saved search.
- The item reflects a system action such as a completed download.

Installing a public connector does not automatically enable its feed.

## Feed sections

### Continue

Horizontal shelf.

Card content:

- Thumbnail
- Title
- Progress
- Remaining time or pages
- Resume action
- Remove from Continue

### For You

Optional local-only recommendations.

Possible signals:

- Private tags
- Creator
- Series
- Media type
- Completion history
- Collection membership

Requirements:

- Disabled by default in private-first profiles
- Explainable
- Resettable
- No cloud training
- No cross-user profiling

### New from Followed

Updates from:

- Series
- Creators
- Saved searches
- Personal servers
- Pinned public sources

### Recent Local Additions

Items discovered by local scan or import.

### Activity

- Downloads complete
- Index complete
- Source requires authentication
- Storage near quota
- Plugin update available

## Feed card actions

- Open
- Resume
- Add to queue
- Favourite
- Add to collection
- Mark seen
- Hide card
- Mute source
- Unfollow
- Block
- Open source
- Explain why shown

## Feed controls

Top-level tabs:

```text
All
Local
Sources
Chapters
Downloads
Notices
```

Secondary controls:

- Sort by relevance
- Sort by newest
- Compact
- Comfortable
- Refresh
- Mark all seen
- Manage feed

## Feed settings

Per source:

- Show on Home
- Show thumbnails
- Blur initially
- Include recent
- Include popular
- Include followed only
- Include saved-search matches
- Maximum cards per refresh
- Notification behavior

Global:

- Maximum feed size
- Hide seen items
- Keep seen items for a set period
- Enable local recommendations
- Show source notices
- Show download activity
- Show indexing activity

## Privacy

Private session behavior:

- No feed-view history
- No new recommendation training
- Session-only external thumbnails
- Dismissed cards reset after session unless explicitly saved
- Continue updates only when the user chooses to save progress

## Feed ranking transparency

Each card can expose “Why shown?”

Examples:

- Added to your local library today
- New chapter in a followed series
- Matches saved search “Short audio”
- From pinned source
- Download completed
- Continue from 42%

## Feed failure states

- Source unavailable
- Rate limited
- Authentication required
- Partial results
- Connector update required
- Feed disabled
- Offline

Failures appear as compact notices, not empty media cards.
