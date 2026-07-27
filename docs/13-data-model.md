# Domain and Data Model

## Core entities

### MediaItem

Represents one logical work or media entry.

```text
MediaItem
- id: UUID
- media_type
- title
- description
- language
- rating_classification   # source/content rating classification, distinct from UserState.rating (the user's personal rating)
- published_at
- discovered_at
- updated_at
- creator_ids[]
- series_id?
- source_refs[]
- variant_ids[]
- tag_ids[]
- category_ids[]
- act_tag_ids[]
- genre_tag_ids[]
- production_tag_ids[]
- safety_status
- visibility_state
- blur_policy_id?
- visual_orientation?
- sexual_orientation_categories[]?
- participant_composition[]?
- gender_identity_categories[]?
- canonical_fingerprint?
- metadata_json
```

The classification fields (`category_ids[]`, `act_tag_ids[]`, `genre_tag_ids[]`, `production_tag_ids[]`, `visibility_state`, `blur_policy_id`, `visual_orientation`, `sexual_orientation_categories[]`, `participant_composition[]`, `gender_identity_categories[]`) are optional and may come from source metadata, local mapping, or user edits. Real-person identity or orientation must not be inferred from appearance or scene participation. See [00 — Glossary](00-glossary.md) for definitions of these taxonomy fields.

### SourceReference

Links an item to a source.

```text
SourceReference
- id
- item_id
- source_id
- source_item_id
- canonical_url
- original_title
- original_description
- original_tags[]
- access_state
- last_checked_at
- deleted_at?
```

### MediaVariant

Represents an actual playable, viewable, readable, cached, or downloadable representation.

```text
MediaVariant
- id
- item_id
- source_ref_id?
- local_path?
- remote_url?
- mime_type
- format
- width?
- height?
- duration_ms?
- bitrate?
- file_size?
- quality_label?
- language?
- expires_at?
- download_permitted
- cache_permitted
- checksum?
```

### UserState

```text
UserState
- item_id
- favorite
- rating?   # the user's personal rating, distinct from MediaItem.rating_classification
- viewed
- completed
- progress_type
- progress_value
- last_opened_at?
- queued_at?
- notes?
- private_tags[]
```

### Collection

```text
Collection
- id
- name
- description
- collection_type: manual|smart|system
- query?
- sort_mode
- cover_item_id?
- created_at
- updated_at
```

### Progress

Progress needs a format-specific representation:

```text
video/audio: milliseconds
story: character offset plus chapter
comic: page index plus intra-page position
gallery: item index
image: viewed boolean
```

Store normalized percentage for display but retain native position for accuracy.

## Media-type submodels

### Gallery

```text
Gallery
- id
- parent_item_id
- ordered_child_ids[]
- cover_child_id?
```

### Series and chapter

```text
Series
- id
- title
- creator_ids[]
- source_refs[]

Chapter
- id
- series_id
- chapter_number?
- volume_number?
- title
- ordered_page_ids[]
```

### Story document

```text
StoryDocument
- item_id
- format
- sanitized_content_location
- chapter_map
- text_index_location
```

## Tag model

```text
Tag
- id
- namespace
- normalized_value
- display_value
- aliases[]
- safety_classification?
```

Namespaces:

- creator
- character
- series
- genre
- format
- language
- source
- user
- technical

Preserve original source tags separately from normalized tags.

## Blocking model

```text
BlockRule
- id
- rule_type
- target
- scope
- reason?
- created_at
- enabled
```

Rule types:

- exact_item
- source
- creator
- series
- tag
- domain
- file_hash
- perceptual_hash
- query

Scope:

- all
- local
- external
- selected_sources

## Download model

```text
Download
- id
- item_id
- variant_id
- state
- destination
- bytes_total?
- bytes_received
- checksum_state
- retry_count
- created_at
- started_at?
- completed_at?
- failure_code?
```

## Source and connector model

```text
Source
- id
- connector_id
- display_name
- enabled
- configuration_json
- credential_ref?
- health_state
- last_health_check?
- capability_snapshot_json
```

## Database guidance

- Use foreign keys.
- Use WAL mode where appropriate.
- Use migrations with explicit version numbers.
- Keep secrets outside SQLite.
- Store large content and thumbnails as files, not database blobs.
- Use FTS tables for searchable text.
- Record local metadata overrides separately from source values.
- Use tombstones for deleted source references when deletion history matters.
