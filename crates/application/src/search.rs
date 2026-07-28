//! Compiles a `domain::SearchQuery` AST into parameterized SQL.
//!
//! Safety is structural, not escaping-based: field names are chosen by a
//! closed `match` over `domain::SearchField` — a column name string is
//! never built from user input — and every value is bound as a `?`
//! parameter, never `format!`-concatenated into the SQL text. Free-text
//! and text-field values are additionally wrapped as escaped FTS5 string
//! literals before binding, so a search value can't reinterpret FTS5's
//! own query mini-language.
//!
//! Scoped to the 14 fields in `domain::search_query` — see that module's
//! doc comment for what's excluded and why.

use domain::{Clause, FieldFilter, FilterValue, MediaType, Predicate, SearchField, SearchQuery};
use rusqlite::{params, ToSql};
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::media_classification::media_type_from_str;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub schema_version: u32,
    pub query: String,
    /// Always `false` this milestone — nothing external to time out yet.
    /// Kept in the shape from `docs/10-cli.md`'s JSON example for
    /// forward-compatibility once connectors exist.
    pub partial: bool,
    pub total: i64,
    pub items: Vec<SearchHit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub item_id: String,
    pub title: String,
    pub media_type: MediaType,
    pub favorite: bool,
}

pub struct SearchService;

impl SearchService {
    pub fn search(
        ctx: &AppContext,
        raw_query: &str,
        limit: u32,
        offset: u32,
    ) -> Result<SearchResults> {
        let query = domain::parse_search_query(raw_query)
            .map_err(|e| AppError::InvalidQuery(e.to_string()))?;
        let (where_sql, params) = compile_query(&query)?;

        let sql = format!(
            "SELECT DISTINCT media_items.id, media_items.title, media_items.media_type, COALESCE(user_state.favorite, 0) AS favorite
             FROM media_items
             LEFT JOIN user_state ON user_state.item_id = media_items.id
             WHERE {where_sql}
             ORDER BY media_items.discovered_at DESC
             LIMIT ? OFFSET ?"
        );
        let count_sql = format!(
            "SELECT COUNT(DISTINCT media_items.id)
             FROM media_items
             LEFT JOIN user_state ON user_state.item_id = media_items.id
             WHERE {where_sql}"
        );

        let conn = ctx.db.connection();

        let count_param_refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, count_param_refs.as_slice(), |row| row.get(0))
            .map_err(database::DatabaseError::from)?;

        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;
        let mut select_param_refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        select_param_refs.push(&limit_i64);
        select_param_refs.push(&offset_i64);

        let mut stmt = conn.prepare(&sql).map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map(select_param_refs.as_slice(), |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let media_type_str: String = row.get(2)?;
                let favorite: bool = row.get(3)?;
                Ok(SearchHit {
                    item_id: id,
                    title,
                    media_type: media_type_from_str(&media_type_str).unwrap_or(MediaType::Other),
                    favorite,
                })
            })
            .map_err(database::DatabaseError::from)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(database::DatabaseError::from)?);
        }

        Ok(SearchResults {
            schema_version: 1,
            query: raw_query.to_string(),
            partial: false,
            total,
            items,
        })
    }

    /// Items with recent playback/reading activity that isn't yet
    /// complete, most-recently-opened first — powers the Home screen's
    /// "Continue" section. Not expressible through the search DSL
    /// (ordering by `last_opened_at` and a "has a value" filter on it
    /// aren't fields the query language exposes), so this is a
    /// dedicated query rather than a `SearchService::search` call.
    pub fn continue_items(ctx: &AppContext, limit: u32) -> Result<Vec<SearchHit>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT media_items.id, media_items.title, media_items.media_type, user_state.favorite
                 FROM media_items
                 JOIN user_state ON user_state.item_id = media_items.id
                 WHERE user_state.last_opened_at IS NOT NULL AND user_state.completed = 0
                 ORDER BY user_state.last_opened_at DESC
                 LIMIT ?1",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map(params![limit], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let media_type_str: String = row.get(2)?;
                let favorite: bool = row.get(3)?;
                Ok(SearchHit {
                    item_id: id,
                    title,
                    media_type: media_type_from_str(&media_type_str).unwrap_or(MediaType::Other),
                    favorite,
                })
            })
            .map_err(database::DatabaseError::from)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(items)
    }
}

fn compile_query(query: &SearchQuery) -> Result<(String, Vec<Box<dyn ToSql>>)> {
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut fragments = Vec::new();
    for clause in &query.clauses {
        fragments.push(compile_clause(clause, &mut params)?);
    }
    let sql = if fragments.is_empty() {
        "1 = 1".to_string()
    } else {
        fragments.join(" AND ")
    };
    Ok((sql, params))
}

fn compile_clause(clause: &Clause, params: &mut Vec<Box<dyn ToSql>>) -> Result<String> {
    match clause {
        Clause::FreeText(text) => Ok(compile_free_text(text, params)),
        Clause::Field(filter) => compile_field_filter(filter, params),
        Clause::Or(members) => {
            let mut parts = Vec::new();
            for member in members {
                parts.push(compile_clause(member, params)?);
            }
            Ok(format!("({})", parts.join(" OR ")))
        }
    }
}

fn compile_free_text(text: &str, params: &mut Vec<Box<dyn ToSql>>) -> String {
    params.push(Box::new(fts5_quote(text)));
    "media_items.rowid IN (SELECT rowid FROM media_items_fts WHERE media_items_fts MATCH ?)"
        .to_string()
}

/// Wraps a value as an FTS5 quoted-phrase literal, doubling embedded
/// quotes per FTS5's own escaping rules.
fn fts5_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Column-scoped FTS5 match, e.g. `title:"phrase"` — restricts the match
/// to a single indexed column rather than any of them.
fn compile_text_match_fts(
    column: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    let text = match predicate {
        Predicate::Equals(FilterValue::Text(s)) => s,
        _ => {
            return Err(AppError::InvalidQuery(format!(
                "{column} only supports equality/text matches"
            )))
        }
    };
    params.push(Box::new(format!("{column}:{}", fts5_quote(text))));
    Ok(
        "media_items.rowid IN (SELECT rowid FROM media_items_fts WHERE media_items_fts MATCH ?)"
            .to_string(),
    )
}

fn compile_field_filter(filter: &FieldFilter, params: &mut Vec<Box<dyn ToSql>>) -> Result<String> {
    let inner = match filter.field {
        SearchField::Type => compile_equals_text("media_items.media_type", &filter.predicate, params)?,
        SearchField::Title => compile_text_match_fts("title", &filter.predicate, params)?,
        SearchField::Description => compile_text_match_fts("description", &filter.predicate, params)?,
        SearchField::Tag => compile_equals_text_exists(
            "EXISTS (SELECT 1 FROM media_item_tags mit JOIN tags t ON t.id = mit.tag_id WHERE mit.item_id = media_items.id AND t.normalized_value = ?)",
            &filter.predicate,
            params,
        )?,
        SearchField::Favorite => compile_bool_column("COALESCE(user_state.favorite, 0)", &filter.predicate, params)?,
        SearchField::Viewed => compile_bool_column("COALESCE(user_state.viewed, 0)", &filter.predicate, params)?,
        SearchField::Completed => compile_bool_column("COALESCE(user_state.completed, 0)", &filter.predicate, params)?,
        SearchField::Rating => compile_comparable("user_state.rating", &filter.predicate, params)?,
        SearchField::Local => compile_local(&filter.predicate)?,
        SearchField::Width => compile_variant_numeric("width", &filter.predicate, params)?,
        SearchField::Height => compile_variant_numeric("height", &filter.predicate, params)?,
        // Milestone B doesn't populate duration_ms (needs FFmpeg probing,
        // deferred to Milestone C) — this field simply matches nothing
        // yet, which is correct, not broken.
        SearchField::Duration => compile_variant_numeric("duration_ms", &filter.predicate, params)?,
        SearchField::Added => compile_date_column("media_items.discovered_at", &filter.predicate, params)?,
        SearchField::Date => compile_date_column("media_items.published_at", &filter.predicate, params)?,
        SearchField::Collection => compile_equals_text_exists(
            "EXISTS (SELECT 1 FROM collection_items ci JOIN collections c ON c.id = ci.collection_id WHERE ci.item_id = media_items.id AND c.name = ?)",
            &filter.predicate,
            params,
        )?,
    };
    Ok(if filter.negated {
        format!("NOT ({inner})")
    } else {
        inner
    })
}

fn compile_equals_text(
    column_expr: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    match predicate {
        Predicate::Equals(FilterValue::Text(s)) => {
            params.push(Box::new(s.clone()));
            Ok(format!("{column_expr} = ?"))
        }
        _ => Err(AppError::InvalidQuery(format!(
            "{column_expr} only supports equality matches"
        ))),
    }
}

/// For fields rendered as an `EXISTS (... = ?)` subquery template with
/// exactly one `?` placeholder.
fn compile_equals_text_exists(
    template: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    match predicate {
        Predicate::Equals(FilterValue::Text(s)) => {
            params.push(Box::new(s.clone()));
            Ok(template.to_string())
        }
        _ => Err(AppError::InvalidQuery(
            "this field only supports equality matches".to_string(),
        )),
    }
}

fn compile_bool_column(
    column_expr: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    match predicate {
        Predicate::Equals(FilterValue::Bool(b)) => {
            params.push(Box::new(*b as i64));
            Ok(format!("{column_expr} = ?"))
        }
        _ => Err(AppError::InvalidQuery(
            "boolean fields only support true/false".to_string(),
        )),
    }
}

fn compile_local(predicate: &Predicate) -> Result<String> {
    match predicate {
        Predicate::Equals(FilterValue::Bool(true)) => Ok(
            "EXISTS (SELECT 1 FROM media_variants mv WHERE mv.item_id = media_items.id AND mv.local_path IS NOT NULL)"
                .to_string(),
        ),
        Predicate::Equals(FilterValue::Bool(false)) => Ok(
            "NOT EXISTS (SELECT 1 FROM media_variants mv WHERE mv.item_id = media_items.id AND mv.local_path IS NOT NULL)"
                .to_string(),
        ),
        _ => Err(AppError::InvalidQuery("local only supports true/false".to_string())),
    }
}

fn compile_variant_numeric(
    column: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    let cmp = compile_comparable(&format!("mv.{column}"), predicate, params)?;
    Ok(format!(
        "EXISTS (SELECT 1 FROM media_variants mv WHERE mv.item_id = media_items.id AND {cmp})"
    ))
}

/// Compares against just the `YYYY-MM-DD` prefix of an RFC3339 column, so
/// calendar-date queries match regardless of stored time-of-day.
fn compile_date_column(
    column: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    compile_comparable(&format!("substr({column}, 1, 10)"), predicate, params)
}

fn compile_comparable(
    column_expr: &str,
    predicate: &Predicate,
    params: &mut Vec<Box<dyn ToSql>>,
) -> Result<String> {
    Ok(match predicate {
        Predicate::Equals(v) => {
            params.push(value_to_sql(v));
            format!("{column_expr} = ?")
        }
        Predicate::LessThan(v) => {
            params.push(value_to_sql(v));
            format!("{column_expr} < ?")
        }
        Predicate::GreaterThan(v) => {
            params.push(value_to_sql(v));
            format!("{column_expr} > ?")
        }
        Predicate::Range(lo, hi) => {
            params.push(value_to_sql(lo));
            params.push(value_to_sql(hi));
            format!("{column_expr} BETWEEN ? AND ?")
        }
    })
}

fn value_to_sql(v: &FilterValue) -> Box<dyn ToSql> {
    match v {
        FilterValue::Text(s) => Box::new(s.clone()),
        FilterValue::Bool(b) => Box::new(*b as i64),
        FilterValue::Number(n) => Box::new(*n),
        FilterValue::DurationMs(ms) => Box::new(*ms),
        FilterValue::Date(d) => Box::new(format!(
            "{:04}-{:02}-{:02}",
            d.year(),
            d.month() as u8,
            d.day()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn insert_item(ctx: &AppContext, title: &str, media_type: &str) -> String {
        let id = domain::ItemId::new().to_string();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, ?2, ?3, 'unrated', datetime('now'), datetime('now'))",
                params![id, media_type, title],
            )
            .unwrap();
        id
    }

    #[test]
    fn filters_by_type() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_item(&ctx, "A Video", "video");
        insert_item(&ctx, "An Image", "image");

        let results = SearchService::search(&ctx, "type:video", 50, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].title, "A Video");
    }

    #[test]
    fn free_text_matches_title_via_fts() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_item(&ctx, "Searchable Title", "video");
        insert_item(&ctx, "Unrelated", "video");

        let results = SearchService::search(&ctx, "Searchable", 50, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].title, "Searchable Title");
    }

    #[test]
    fn filters_by_favorite() {
        let ctx = AppContext::open_in_memory().unwrap();
        let id = insert_item(&ctx, "Fav", "video");
        insert_item(&ctx, "NotFav", "video");
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, favorite) VALUES (?1, 1)",
                params![id],
            )
            .unwrap();

        let results = SearchService::search(&ctx, "favorite:true", 50, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].title, "Fav");
    }

    #[test]
    fn filters_by_tag() {
        let ctx = AppContext::open_in_memory().unwrap();
        let id = insert_item(&ctx, "Tagged", "video");
        insert_item(&ctx, "Untagged", "video");

        ctx.db
            .connection()
            .execute(
                "INSERT INTO tags (id, namespace, normalized_value, display_value) VALUES ('t1', 'user', 'blue', 'Blue')",
                [],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_item_tags (item_id, tag_id) VALUES (?1, 't1')",
                params![id],
            )
            .unwrap();

        let results = SearchService::search(&ctx, "tag:blue", 50, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].title, "Tagged");
    }

    #[test]
    fn negation_excludes_matches() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_item(&ctx, "A Video", "video");
        insert_item(&ctx, "An Image", "image");

        let results = SearchService::search(&ctx, "-type:video", 50, 0).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].title, "An Image");
    }

    #[test]
    fn or_group_matches_either_side() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_item(&ctx, "Alpha", "video");
        insert_item(&ctx, "Beta", "video");
        insert_item(&ctx, "Gamma", "video");

        let results = SearchService::search(&ctx, "(Alpha OR Beta)", 50, 0).unwrap();
        assert_eq!(results.total, 2);
    }

    #[test]
    fn limit_and_offset_paginate() {
        let ctx = AppContext::open_in_memory().unwrap();
        for i in 0..5 {
            insert_item(&ctx, &format!("Item {i}"), "video");
        }

        let page = SearchService::search(&ctx, "type:video", 2, 2).unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 2);
    }

    #[test]
    fn continue_items_excludes_unopened_and_completed_items() {
        let ctx = AppContext::open_in_memory().unwrap();
        let never_opened = insert_item(&ctx, "Never Opened", "video");
        let in_progress = insert_item(&ctx, "In Progress", "video");
        let completed = insert_item(&ctx, "Completed", "video");
        for (id, completed_flag) in [
            (never_opened.clone(), false),
            (in_progress.clone(), false),
            (completed.clone(), true),
        ] {
            if id == never_opened {
                continue;
            }
            ctx.db
                .connection()
                .execute(
                    "INSERT INTO user_state (item_id, last_opened_at, completed) VALUES (?1, datetime('now'), ?2)",
                    params![id, completed_flag as i64],
                )
                .unwrap();
        }

        let results = SearchService::continue_items(&ctx, 50).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "In Progress");
    }

    #[test]
    fn continue_items_orders_most_recently_opened_first() {
        let ctx = AppContext::open_in_memory().unwrap();
        let older = insert_item(&ctx, "Older", "video");
        let newer = insert_item(&ctx, "Newer", "video");
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, last_opened_at, completed) VALUES (?1, '2020-01-01T00:00:00Z', 0)",
                params![older],
            )
            .unwrap();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO user_state (item_id, last_opened_at, completed) VALUES (?1, '2024-01-01T00:00:00Z', 0)",
                params![newer],
            )
            .unwrap();

        let results = SearchService::continue_items(&ctx, 50).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Newer");
        assert_eq!(results[1].title, "Older");
    }

    #[test]
    fn unknown_field_is_an_invalid_query_error() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = SearchService::search(&ctx, "bogus:value", 50, 0).unwrap_err();
        assert!(matches!(err, AppError::InvalidQuery(_)));
    }

    #[test]
    fn quoted_sql_like_value_is_a_literal_search_term_not_executed() {
        let ctx = AppContext::open_in_memory().unwrap();
        insert_item(&ctx, "Normal Title", "video");
        insert_item(&ctx, "'; DROP TABLE media_items; --", "video");

        let results =
            SearchService::search(&ctx, r#"title:"'; DROP TABLE media_items; --""#, 50, 0).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].title, "'; DROP TABLE media_items; --");

        // The schema must be completely untouched.
        let count: i64 = ctx
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
