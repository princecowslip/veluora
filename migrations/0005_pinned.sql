-- Milestone G: a "pinned" flag on user_state, used as a cache-eviction
-- exemption (docs/17-downloads-cache-storage.md's "Never remove pinned"
-- quota policy) — not a remote-download concept. Local library files
-- already are the permanent originals; pinning just protects an item's
-- *generated* cache artifacts (thumbnails) from quota-driven eviction.

ALTER TABLE user_state ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
