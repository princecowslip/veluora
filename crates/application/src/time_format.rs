//! Consistent RFC3339 round-tripping of `OffsetDateTime` through SQLite
//! TEXT columns. Milestone A never stored a timestamp in a column that
//! got read back into `OffsetDateTime` (only `datetime('now')` via SQL
//! for tracking-only columns) — this is the first place that needs a
//! defined, symmetric format.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn to_rfc3339(dt: OffsetDateTime) -> String {
    dt.format(&Rfc3339)
        .expect("OffsetDateTime always formats as RFC3339")
}

pub fn from_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}
