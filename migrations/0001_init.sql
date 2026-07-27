-- Milestone A schema: entities from docs/13-data-model.md.
--
-- Large content and thumbnails are never stored here — only local paths
-- and metadata (per the "Database guidance" section of that doc). Secrets
-- never enter this database; `sources.credential_ref` is an opaque handle
-- into the OS credential store, not a raw secret.

CREATE TABLE series (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL
);

CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    connector_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    configuration_json TEXT NOT NULL DEFAULT '{}',
    credential_ref TEXT,
    health_state TEXT NOT NULL DEFAULT 'unknown',
    last_health_check TEXT,
    capability_snapshot_json TEXT
);

CREATE TABLE media_items (
    id TEXT PRIMARY KEY,
    media_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    language TEXT,
    rating_classification TEXT NOT NULL DEFAULT 'unrated',
    published_at TEXT,
    discovered_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    series_id TEXT REFERENCES series(id) ON DELETE SET NULL,
    safety_status TEXT NOT NULL DEFAULT 'unreviewed',
    visibility_state TEXT NOT NULL DEFAULT 'visible',
    blur_policy_id TEXT,
    visual_orientation TEXT,
    sexual_orientation_categories TEXT,
    participant_composition TEXT,
    gender_identity_categories TEXT,
    canonical_fingerprint TEXT,
    metadata_json TEXT
);

CREATE INDEX idx_media_items_series ON media_items(series_id);
CREATE INDEX idx_media_items_media_type ON media_items(media_type);

CREATE TABLE source_references (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    source_item_id TEXT NOT NULL,
    canonical_url TEXT,
    original_title TEXT,
    original_description TEXT,
    original_tags TEXT,
    access_state TEXT NOT NULL DEFAULT 'available',
    last_checked_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX idx_source_references_item ON source_references(item_id);
CREATE INDEX idx_source_references_source ON source_references(source_id);

CREATE TABLE media_variants (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    source_ref_id TEXT REFERENCES source_references(id) ON DELETE SET NULL,
    local_path TEXT,
    remote_url TEXT,
    mime_type TEXT NOT NULL,
    format TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    duration_ms INTEGER,
    bitrate INTEGER,
    file_size INTEGER,
    quality_label TEXT,
    language TEXT,
    expires_at TEXT,
    download_permitted INTEGER NOT NULL DEFAULT 0,
    cache_permitted INTEGER NOT NULL DEFAULT 0,
    checksum TEXT
);

CREATE INDEX idx_media_variants_item ON media_variants(item_id);

CREATE TABLE user_state (
    item_id TEXT PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
    favorite INTEGER NOT NULL DEFAULT 0,
    rating INTEGER,
    viewed INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    progress_json TEXT,
    last_opened_at TEXT,
    queued_at TEXT,
    notes TEXT,
    private_tags TEXT
);

CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    collection_type TEXT NOT NULL,
    query TEXT,
    sort_mode TEXT NOT NULL DEFAULT 'added_desc',
    cover_item_id TEXT REFERENCES media_items(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE collection_items (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    added_at TEXT NOT NULL,
    PRIMARY KEY (collection_id, item_id)
);

-- Normalized tags. Original, unnormalized source tags are preserved
-- separately on `source_references.original_tags` per the data model doc.
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    normalized_value TEXT NOT NULL,
    display_value TEXT NOT NULL,
    aliases TEXT,
    safety_classification TEXT,
    UNIQUE (namespace, normalized_value)
);

CREATE TABLE media_item_tags (
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    relation TEXT NOT NULL DEFAULT 'tag',
    PRIMARY KEY (item_id, tag_id, relation)
);

CREATE TABLE block_rules (
    id TEXT PRIMARY KEY,
    rule_type TEXT NOT NULL,
    target TEXT NOT NULL,
    scope TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE downloads (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    variant_id TEXT NOT NULL REFERENCES media_variants(id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'queued',
    destination TEXT NOT NULL,
    bytes_total INTEGER,
    bytes_received INTEGER NOT NULL DEFAULT 0,
    checksum_state TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    failure_code TEXT
);

-- Full-text search over title/description (ADR-003, Workstream 3).
-- External-content FTS5 table kept in sync with media_items via triggers.
CREATE VIRTUAL TABLE media_items_fts USING fts5(
    title,
    description,
    content = 'media_items',
    content_rowid = 'rowid'
);

CREATE TRIGGER media_items_fts_ai AFTER INSERT ON media_items BEGIN
    INSERT INTO media_items_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

CREATE TRIGGER media_items_fts_ad AFTER DELETE ON media_items BEGIN
    INSERT INTO media_items_fts(media_items_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
END;

CREATE TRIGGER media_items_fts_au AFTER UPDATE ON media_items BEGIN
    INSERT INTO media_items_fts(media_items_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO media_items_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;
