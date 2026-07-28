-- Milestone D: a minimal key-value store for GUI/session-level
-- preferences that don't fit the existing domain model (onboarding
-- completion, theme choice, external player path, lock password hash,
-- etc.). Typed accessors live in application::SettingsService rather
-- than scattering raw key strings across callers.

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
