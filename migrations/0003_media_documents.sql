-- Milestone C: media experience. Persists the StoryDocument domain
-- submodel (docs/13-data-model.md), which existed in the domain crate
-- since Milestone A/B scaffolding but had nowhere to live, plus a
-- comic/manga page count populated at scan time.

CREATE TABLE story_documents (
    item_id TEXT PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    sanitized_content_location TEXT NOT NULL,
    chapter_map TEXT NOT NULL,
    text_index_location TEXT
);

-- Page count for comic/manga archives, populated by the scanner's
-- CBZ page-listing pass (crates/media::archive::list_pages). NULL until
-- probed, and left NULL for non-archive variants.
ALTER TABLE media_variants ADD COLUMN page_count INTEGER;
