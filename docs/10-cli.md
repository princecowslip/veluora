# CLI Specification

## Command name

The binary name is `veloura`, matching the product name used across the GUI, TUI, and `veloura://` deep-link scheme (see [05 — Information Architecture](05-information-architecture.md)).

## Design principles

- Stable output
- Scriptable behavior
- Explicit destructive operations
- JSON support
- Useful exit codes
- No secrets in command history
- Equivalent core behavior to GUI and TUI

## Global syntax

```bash
veloura [global-options] <command> [subcommand] [options]
```

Global options:

```text
--config PATH
--profile NAME
--output text|json|jsonl|table
--no-color
--quiet
--verbose
--offline
--private
--timeout DURATION
```

## Commands

### Library

```bash
veloura library add /media/library
veloura library list
veloura library scan
veloura library scan --path /media/library
veloura library remove LIBRARY_ID
veloura library status
```

### Search and browse

```bash
veloura search 'type:video duration:<20m -tag:blocked'
veloura search 'creator:"Example"' --source local --output json
veloura browse --source SOURCE_ID --type image
veloura discover 'query'
veloura saved-search list
veloura saved-search create NAME --query '...'
```

`veloura discover` is real — it aggregates the local library with every
enabled connector source in one call (`application::DiscoverService`).
`saved-search` is not implemented anywhere yet.

### Items

```bash
veloura item show ITEM_ID
veloura item open ITEM_ID
veloura item play ITEM_ID
veloura item read ITEM_ID
veloura item reveal ITEM_ID
veloura item edit ITEM_ID --title '...'
```

### User state

```bash
veloura favorite add ITEM_ID
veloura favorite remove ITEM_ID
veloura rating set ITEM_ID 4
veloura viewed mark ITEM_ID
veloura progress set ITEM_ID --position 00:12:45
veloura tag add ITEM_ID private-tag
veloura note edit ITEM_ID
```

### Collections

```bash
veloura collection create 'Later'
veloura collection list
veloura collection add ITEM_ID --to COLLECTION_ID
veloura collection remove ITEM_ID --from COLLECTION_ID
veloura collection export COLLECTION_ID
```

### Queue

No standalone queue concept exists anywhere in the domain, application, or
CLI layer — queueing is part of the Downloads command group below, not a
separate command.

### Sources

```bash
veloura source list
veloura source add CONNECTOR_ID --name NAME --config CONFIG_JSON
veloura source remove SOURCE_ID
veloura source enable SOURCE_ID
veloura source disable SOURCE_ID
veloura source health-check SOURCE_ID
veloura source browse SOURCE_ID --route ROUTE
veloura source import SOURCE_ID --item SOURCE_ITEM_ID
```

Connector configuration (including any API key or Basic-auth credentials)
is currently passed as opaque `configuration_json` — there is no separate
`configure`/`login`/`logout`/`test` step or credential-manager integration
yet. Authentication secrets should still be read from a secure prompt or
file descriptor rather than a normal command argument where possible.

### Downloads

```bash
veloura download add ITEM_ID
veloura download list
veloura download pause DOWNLOAD_ID
veloura download resume DOWNLOAD_ID
veloura download cancel DOWNLOAD_ID
veloura download remove DOWNLOAD_ID
veloura download pin DOWNLOAD_ID
veloura download eligibility ITEM_ID
veloura download quota
veloura download enforce-quota
veloura db cache-status
veloura db cache-quota
veloura db cache-enforce-quota
```

### Privacy and safety

```bash
veloura lock
veloura privacy status
veloura history clear --searches
veloura history clear --viewing
veloura history clear --all
veloura block add tag:example
veloura block list
veloura block remove BLOCK_ID
```

Destructive clearing requires either an interactive confirmation or `--yes`.

### Plugins

Shipped today (Milestone H — governance/sandbox infrastructure only, no
real third-party plugin exists to install yet):

```bash
veloura plugin validate MANIFEST_PATH
veloura plugin registry-add MANIFEST_PATH
veloura plugin registry-list
veloura plugin registry-set-status PLUGIN_ID STATUS
```

Not yet implemented — a future plugin-marketplace-style flow:

```bash
veloura plugin inspect PLUGIN_ID
veloura plugin install PACKAGE
veloura plugin disable PLUGIN_ID
veloura plugin remove PLUGIN_ID
veloura plugin permissions PLUGIN_ID
```

### Diagnostics

```bash
veloura doctor
veloura doctor --source SOURCE_ID
veloura db check
veloura thumbnails repair
veloura support-bundle create --redacted
```

## Query language

Supported operators:

```text
field:value
-field:value
field:>value
field:<value
field:value..value
"exact phrase"
term1 OR term2
(term1 OR term2) field:value
```

Fields:

```text
type
source
creator
series
tag
language
duration
pages
width
height
date
added
viewed
favorite
rating
downloaded
local
```

## Exit codes

```text
0   Success
1   General failure
2   Invalid arguments
3   Not found
4   Authentication required
5   Permission denied
6   Network failure
7   Rate limited
8   Unsupported capability
9   Partial success
10  Safety block
11  Database failure
12  Configuration failure
```

## JSON output

All JSON objects should include a schema version.

Example:

```json
{
  "schema_version": 1,
  "query": "type:audio",
  "partial": false,
  "items": []
}
```

## Shell completion

Generate completion for:

- Bash
- Zsh
- Fish
- PowerShell

Completions may include command names, option names, source IDs, and collection IDs, but should not expose sensitive titles by default.
