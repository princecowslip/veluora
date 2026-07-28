-- Milestone B: library roots for scanning, plus hashing/move-detection
-- support on media_variants. docs/13-data-model.md doesn't define a
-- LibraryRoot entity (it predates scanning); this is the minimal
-- reasonable shape per Workstream 4's acceptance criteria in
-- docs/46-implementation-plan.md.

CREATE TABLE library_roots (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    display_name TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    last_scanned_at TEXT
);

-- content_hash powers move-detection and future dedup identification.
-- last_seen_at lets a scan tell "still present" apart from "not
-- revisited this walk" without deleting anything. library_root_id scopes
-- a re-scan to the variants it owns, bounding missing-file detection to
-- one root at a time. mtime_unix backs the scanner's unchanged-file fast
-- path (skip re-hashing when size and mtime both match).
ALTER TABLE media_variants ADD COLUMN content_hash TEXT;
ALTER TABLE media_variants ADD COLUMN last_seen_at TEXT;
ALTER TABLE media_variants ADD COLUMN library_root_id TEXT REFERENCES library_roots(id) ON DELETE SET NULL;
ALTER TABLE media_variants ADD COLUMN mtime_unix INTEGER;

CREATE INDEX idx_media_variants_content_hash ON media_variants(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX idx_media_variants_local_path ON media_variants(local_path) WHERE local_path IS NOT NULL;
CREATE INDEX idx_media_variants_library_root ON media_variants(library_root_id);
