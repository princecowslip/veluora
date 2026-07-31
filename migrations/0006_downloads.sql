-- Milestone J: Downloads and offline use (Workstream 11). The
-- `downloads` table has existed since migration 0001 as unused
-- scaffolding (docs/13-data-model.md's Download model) — this is that
-- table's first real consumer. Columns added here support cross-source
-- resume validation (etag/last_modified), the pinned/permanent
-- eviction exemption (mirroring migration 0005's user_state.pinned),
-- and a denormalized source_id so eligibility re-checks and quota
-- listing don't need a join through media_variants/source_references
-- every time.

ALTER TABLE downloads ADD COLUMN source_id TEXT REFERENCES sources(id) ON DELETE SET NULL;
ALTER TABLE downloads ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE downloads ADD COLUMN temp_path TEXT;
ALTER TABLE downloads ADD COLUMN expected_checksum TEXT;
ALTER TABLE downloads ADD COLUMN checksum_algorithm TEXT;
ALTER TABLE downloads ADD COLUMN etag TEXT;
ALTER TABLE downloads ADD COLUMN last_modified TEXT;
ALTER TABLE downloads ADD COLUMN updated_at TEXT;

CREATE INDEX idx_downloads_item ON downloads(item_id);
CREATE INDEX idx_downloads_state ON downloads(state);
CREATE INDEX idx_downloads_source ON downloads(source_id);
