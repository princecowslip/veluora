# Source Connector Framework

## Purpose

Connectors translate a source-specific API, feed, or local interface into the common domain model.

A connector is not a general-purpose scraper. It supports named domains and declared capabilities.

## Capability model

```json
{
  "search": true,
  "browse": true,
  "item_details": true,
  "streaming": true,
  "downloads": false,
  "comments": false,
  "authentication": ["oauth"],
  "media_types": ["video", "image"],
  "pagination": "cursor",
  "rate_limit": {
    "requests": 60,
    "period_seconds": 60
  }
}
```

The application must check capabilities before showing actions.

## Connector interface

```text
identify()
capabilities()
configure()
authenticate()
logout()
health_check()
search(query, page)
browse(route, page)
get_item(source_item_id)
get_gallery(source_item_id)
resolve_variants(source_item_id)
get_tags(prefix)
refresh_access(source_item_id)
```

Optional methods are omitted or return an explicit unsupported-capability result.

## Result types

Connector calls should return structured results:

```text
Success<T>
Partial<T>
AuthenticationRequired
RateLimited
UnsupportedQuery
UnsupportedCapability
NotFound
Deleted
BlockedBySource
TemporaryFailure
PermanentFailure
```

Do not represent all failures as empty results.

## Query translation

The application parses the user query into an abstract syntax tree. Each connector declares which nodes it supports.

Example translation report:

```text
Source supports:
- tag inclusion
- tag exclusion
- date sorting

Source does not support:
- duration filter
- OR expressions

Application behavior:
- send supported clauses
- apply duration locally
- mark results as potentially incomplete
```

## Authentication

Supported approaches may include:

- OAuth
- API token
- Cookie supplied directly by the user
- Session token
- Anonymous access

Browser-cookie import is high risk and should be optional, narrowly scoped, and off by default.

Connectors receive temporary credential handles rather than direct access to the global credential store.

## Networking

The connector host provides the HTTP client.

Enforced controls:

- Allowed domains
- HTTPS requirement
- Redirect policy
- Request timeout
- Response-size limit
- Rate limit
- User-agent policy
- Cookie isolation
- DNS and private-network restrictions
- Content-type validation

## Connector manifest

```yaml
id: example.connector
name: Example Connector
version: 1.0.0
api_version: 1
domains:
  - api.example.test
permissions:
  network:
    - api.example.test
  credentials:
    - source_scoped
capabilities:
  - search
  - item_details
  - streaming
media_types:
  - image
  - video
```

## Connector lifecycle

1. Install or enable.
2. Validate manifest.
3. Review permissions.
4. Configure.
5. Authenticate.
6. Run health check.
7. Snapshot capabilities.
8. Execute within host limits.
9. Update or disable independently.

## Reference connectors

Shipped today (`crates/connectors/src/`):

- Local filesystem
- RSS or Atom media feed (`FeedConnector`)
- Generic booru-compatible API, covering both Danbooru's REST/JSON API and
  Gelbooru's DAPI via a `flavor` config field (`BooruConnector`)
- OPDS 1.x catalog, for self-hosted book/comic/manga servers such as
  Komga, Kavita, and Calibre-Web (`OpdsConnector`)

The generic connector should target a documented API family, not arbitrary HTML.

Personal-server connectors beyond OPDS (Jellyfin, Kavita's native API,
Komga's native API, Audiobookshelf, and similar) remain unbuilt — see
`KNOWN_ISSUES.md`.

## Maintenance policy

Official connector status may be:

- Stable
- Beta
- Degraded
- Disabled
- Removed

The application should support a signed revocation list for connectors that become unsafe, compromised, or legally unsuitable.
