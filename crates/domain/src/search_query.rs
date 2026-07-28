//! Local search query grammar: a scoped-down subset of the `field:value`
//! language in `docs/10-cli.md` / `docs/15-search-and-discovery.md`.
//!
//! Pure parsing logic, no I/O — the same "evaluate without presentation
//! code" pattern as [`crate::block_rule::BlockRule::evaluate`]. Scoped to
//! 14 fields that don't depend on connectors or taxonomy not yet
//! populated by anything (see the Milestone B plan for the full field
//! list this deliberately excludes). Supports `field:value`,
//! `-field:value` negation, `field:<value`/`field:>value` comparisons,
//! `field:a..b` ranges, quoted phrases, bareword free text, and one level
//! of `(a OR b)` grouping — no nested parentheses, no escaped quotes.

use std::fmt;

use time::macros::format_description;
use time::Date;

/// A parsed local search query: an implicit AND across all top-level
/// clauses.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    pub clauses: Vec<Clause>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Clause {
    Field(FieldFilter),
    FreeText(String),
    /// One level of `(a OR b OR ...)` grouping.
    Or(Vec<Clause>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldFilter {
    pub field: SearchField,
    pub negated: bool,
    pub predicate: Predicate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Equals(FilterValue),
    LessThan(FilterValue),
    GreaterThan(FilterValue),
    Range(FilterValue, FilterValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    Text(String),
    Bool(bool),
    Number(f64),
    DurationMs(i64),
    Date(Date),
}

/// The closed set of fields this milestone's parser understands. Any
/// other `field:value` token is a parse error naming the field, rather
/// than silently falling back to free text, per `docs/10-cli.md`'s
/// requirement that search errors identify the invalid segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    Type,
    Title,
    Description,
    Tag,
    Favorite,
    Viewed,
    Completed,
    Rating,
    Local,
    Width,
    Height,
    Duration,
    Added,
    Date,
    Collection,
}

const MEDIA_TYPE_NAMES: &[&str] = &[
    "video", "image", "gallery", "audio", "story", "manga", "comic", "other",
];

impl SearchField {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "type" => Self::Type,
            "title" => Self::Title,
            "description" => Self::Description,
            "tag" => Self::Tag,
            "favorite" | "favourite" => Self::Favorite,
            "viewed" => Self::Viewed,
            "completed" => Self::Completed,
            "rating" => Self::Rating,
            "local" => Self::Local,
            "width" => Self::Width,
            "height" => Self::Height,
            "duration" => Self::Duration,
            "added" => Self::Added,
            "date" => Self::Date,
            "collection" => Self::Collection,
            _ => return None,
        })
    }

    fn value_kind(self) -> ValueKind {
        match self {
            Self::Favorite | Self::Viewed | Self::Completed | Self::Local => ValueKind::Bool,
            Self::Rating | Self::Width | Self::Height => ValueKind::Number,
            Self::Duration => ValueKind::Duration,
            Self::Added | Self::Date => ValueKind::Date,
            Self::Type | Self::Title | Self::Description | Self::Tag | Self::Collection => {
                ValueKind::Text
            }
        }
    }
}

impl fmt::Display for SearchField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Type => "type",
            Self::Title => "title",
            Self::Description => "description",
            Self::Tag => "tag",
            Self::Favorite => "favorite",
            Self::Viewed => "viewed",
            Self::Completed => "completed",
            Self::Rating => "rating",
            Self::Local => "local",
            Self::Width => "width",
            Self::Height => "height",
            Self::Duration => "duration",
            Self::Added => "added",
            Self::Date => "date",
            Self::Collection => "collection",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, Copy)]
enum ValueKind {
    Text,
    Bool,
    Number,
    Duration,
    Date,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryParseError {
    pub message: String,
    /// Byte offsets into the original query string.
    pub span: (usize, usize),
}

impl fmt::Display for QueryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at {}..{})", self.message, self.span.0, self.span.1)
    }
}

impl std::error::Error for QueryParseError {}

struct Token {
    text: String,
    start: usize,
    end: usize,
}

/// Splits on whitespace, keeping quoted sections (which may contain
/// spaces) intact, and treating `(`/`)` as standalone tokens even when
/// attached to adjacent text. No escaped-quote support.
fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '(' || c == ')' {
            tokens.push(Token {
                text: c.to_string(),
                start,
                end: start + c.len_utf8(),
            });
            i += 1;
            continue;
        }
        let mut buf = String::new();
        let mut end = start;
        let mut in_quotes = false;
        while i < chars.len() {
            let (idx, ch) = chars[i];
            if ch == '"' {
                in_quotes = !in_quotes;
                buf.push(ch);
                end = idx + ch.len_utf8();
                i += 1;
                continue;
            }
            if !in_quotes && (ch.is_whitespace() || ch == '(' || ch == ')') {
                break;
            }
            buf.push(ch);
            end = idx + ch.len_utf8();
            i += 1;
        }
        tokens.push(Token {
            text: buf,
            start,
            end,
        });
    }
    tokens
}

fn strip_quotes(s: &str) -> Option<String> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

/// Parse a `field:value` search query into a [`SearchQuery`] AST.
pub fn parse(input: &str) -> Result<SearchQuery, QueryParseError> {
    let tokens = tokenize(input);
    let mut clauses = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].text == "(" {
            let (or_clause, next) = parse_or_group(&tokens, i)?;
            clauses.push(or_clause);
            i = next;
        } else if tokens[i].text == ")" {
            return Err(QueryParseError {
                message: "unmatched ')'".to_string(),
                span: (tokens[i].start, tokens[i].end),
            });
        } else {
            clauses.push(parse_atomic(&tokens[i])?);
            i += 1;
        }
    }
    Ok(SearchQuery { clauses })
}

fn parse_or_group(tokens: &[Token], open_idx: usize) -> Result<(Clause, usize), QueryParseError> {
    let mut i = open_idx + 1;
    let mut members = Vec::new();
    loop {
        if i >= tokens.len() {
            return Err(QueryParseError {
                message: "unterminated '(' group".to_string(),
                span: (tokens[open_idx].start, tokens[open_idx].end),
            });
        }
        if tokens[i].text == ")" {
            i += 1;
            break;
        }
        if tokens[i].text == "(" {
            return Err(QueryParseError {
                message: "nested '(' groups are not supported".to_string(),
                span: (tokens[i].start, tokens[i].end),
            });
        }
        members.push(parse_atomic(&tokens[i])?);
        i += 1;
        if i < tokens.len() && tokens[i].text.eq_ignore_ascii_case("OR") {
            i += 1;
        }
    }
    if members.is_empty() {
        return Err(QueryParseError {
            message: "empty '(...)' group".to_string(),
            span: (tokens[open_idx].start, tokens[open_idx].end),
        });
    }
    Ok((Clause::Or(members), i))
}

fn parse_atomic(token: &Token) -> Result<Clause, QueryParseError> {
    if let Some(inner) = strip_quotes(&token.text) {
        return Ok(Clause::FreeText(inner));
    }

    let Some((name_part, rest)) = token.text.split_once(':') else {
        return Ok(Clause::FreeText(token.text.clone()));
    };

    let (negated, field_name) = match name_part.strip_prefix('-') {
        Some(rest_name) => (true, rest_name),
        None => (false, name_part),
    };

    let field = SearchField::from_name(field_name).ok_or_else(|| QueryParseError {
        message: format!("unknown search field '{field_name}'"),
        span: (token.start, token.start + name_part.len()),
    })?;

    let predicate = parse_predicate(field, rest, token)?;
    Ok(Clause::Field(FieldFilter {
        field,
        negated,
        predicate,
    }))
}

fn parse_predicate(
    field: SearchField,
    raw_value: &str,
    token: &Token,
) -> Result<Predicate, QueryParseError> {
    if let Some(unquoted) = strip_quotes(raw_value) {
        return Ok(Predicate::Equals(parse_value(field, &unquoted, token)?));
    }

    // Comparisons/ranges are meaningless for free-text and boolean
    // fields, so those always fall through to a literal Equals below —
    // e.g. `tag:c++` isn't misread as a comparison.
    let supports_operators = !matches!(field.value_kind(), ValueKind::Text | ValueKind::Bool);

    if supports_operators {
        if let Some(rest) = raw_value.strip_prefix('<') {
            return Ok(Predicate::LessThan(parse_value(field, rest, token)?));
        }
        if let Some(rest) = raw_value.strip_prefix('>') {
            return Ok(Predicate::GreaterThan(parse_value(field, rest, token)?));
        }
        if let Some((low, high)) = raw_value.split_once("..") {
            if !low.is_empty() && !high.is_empty() {
                return Ok(Predicate::Range(
                    parse_value(field, low, token)?,
                    parse_value(field, high, token)?,
                ));
            }
        }
    }

    Ok(Predicate::Equals(parse_value(field, raw_value, token)?))
}

fn parse_value(
    field: SearchField,
    raw: &str,
    token: &Token,
) -> Result<FilterValue, QueryParseError> {
    if raw.is_empty() {
        return Err(QueryParseError {
            message: format!("missing value for field '{field}'"),
            span: (token.start, token.end),
        });
    }
    match field.value_kind() {
        ValueKind::Text => {
            if field == SearchField::Type
                && !MEDIA_TYPE_NAMES.contains(&raw.to_ascii_lowercase().as_str())
            {
                return Err(QueryParseError {
                    message: format!("unknown media type '{raw}'"),
                    span: (token.start, token.end),
                });
            }
            Ok(FilterValue::Text(raw.to_string()))
        }
        ValueKind::Bool => match raw.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(FilterValue::Bool(true)),
            "false" | "no" | "0" => Ok(FilterValue::Bool(false)),
            _ => Err(QueryParseError {
                message: format!("expected true/false, got '{raw}'"),
                span: (token.start, token.end),
            }),
        },
        ValueKind::Number => {
            raw.parse::<f64>()
                .map(FilterValue::Number)
                .map_err(|_| QueryParseError {
                    message: format!("expected a number, got '{raw}'"),
                    span: (token.start, token.end),
                })
        }
        ValueKind::Duration => parse_duration_ms(raw)
            .map(FilterValue::DurationMs)
            .ok_or_else(|| QueryParseError {
                message: format!("expected a duration like '20m' or '90s', got '{raw}'"),
                span: (token.start, token.end),
            }),
        ValueKind::Date => {
            let format = format_description!("[year]-[month]-[day]");
            Date::parse(raw, &format)
                .map(FilterValue::Date)
                .map_err(|_| QueryParseError {
                    message: format!("expected a date like 'YYYY-MM-DD', got '{raw}'"),
                    span: (token.start, token.end),
                })
        }
    }
}

/// Supports a single suffixed number: `s`/`m`/`h`, or a bare number of
/// seconds. Compound durations like `1h30m` are not supported this pass.
fn parse_duration_ms(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let (num_part, unit) = match raw.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&raw[..raw.len() - c.len_utf8()], c),
        _ => (raw, 's'),
    };
    let value: f64 = num_part.parse().ok()?;
    let multiplier_ms: f64 = match unit.to_ascii_lowercase() {
        's' => 1_000.0,
        'm' => 60_000.0,
        'h' => 3_600_000.0,
        _ => return None,
    };
    Some((value * multiplier_ms).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_and_free_text_together() {
        // Barewords are AND'd individually, not merged into one phrase —
        // matches typical full-text search semantics.
        let query = parse("type:video favorite:true some words").unwrap();
        assert_eq!(query.clauses.len(), 4);
        assert_eq!(
            query.clauses[0],
            Clause::Field(FieldFilter {
                field: SearchField::Type,
                negated: false,
                predicate: Predicate::Equals(FilterValue::Text("video".to_string())),
            })
        );
        assert_eq!(
            query.clauses[1],
            Clause::Field(FieldFilter {
                field: SearchField::Favorite,
                negated: false,
                predicate: Predicate::Equals(FilterValue::Bool(true)),
            })
        );
        assert_eq!(query.clauses[2], Clause::FreeText("some".to_string()));
        assert_eq!(query.clauses[3], Clause::FreeText("words".to_string()));
    }

    #[test]
    fn parses_negation() {
        let query = parse("-tag:blocked").unwrap();
        assert_eq!(
            query.clauses[0],
            Clause::Field(FieldFilter {
                field: SearchField::Tag,
                negated: true,
                predicate: Predicate::Equals(FilterValue::Text("blocked".to_string())),
            })
        );
    }

    #[test]
    fn parses_numeric_range() {
        let query = parse("width:100..200").unwrap();
        assert_eq!(
            query.clauses[0],
            Clause::Field(FieldFilter {
                field: SearchField::Width,
                negated: false,
                predicate: Predicate::Range(FilterValue::Number(100.0), FilterValue::Number(200.0)),
            })
        );
    }

    #[test]
    fn parses_duration_comparison() {
        let query = parse("duration:<20m").unwrap();
        assert_eq!(
            query.clauses[0],
            Clause::Field(FieldFilter {
                field: SearchField::Duration,
                negated: false,
                predicate: Predicate::LessThan(FilterValue::DurationMs(1_200_000)),
            })
        );
    }

    #[test]
    fn parses_one_level_or_group() {
        let query = parse("(term1 OR term2) type:image").unwrap();
        assert_eq!(query.clauses.len(), 2);
        assert_eq!(
            query.clauses[0],
            Clause::Or(vec![
                Clause::FreeText("term1".to_string()),
                Clause::FreeText("term2".to_string()),
            ])
        );
    }

    #[test]
    fn parses_quoted_phrase_as_free_text() {
        let query = parse(r#""exact phrase""#).unwrap();
        assert_eq!(
            query.clauses[0],
            Clause::FreeText("exact phrase".to_string())
        );
    }

    #[test]
    fn quoted_value_containing_sql_like_text_is_treated_as_literal() {
        let query = parse(r#"title:"'; DROP TABLE media_items; --""#).unwrap();
        assert_eq!(
            query.clauses[0],
            Clause::Field(FieldFilter {
                field: SearchField::Title,
                negated: false,
                predicate: Predicate::Equals(FilterValue::Text(
                    "'; DROP TABLE media_items; --".to_string()
                )),
            })
        );
    }

    #[test]
    fn unknown_field_reports_the_offending_span() {
        let err = parse("nope:value").unwrap_err();
        assert_eq!(err.span, (0, 4));
        assert!(err.message.contains("nope"));
    }

    #[test]
    fn invalid_boolean_value_is_a_parse_error() {
        let err = parse("favorite:maybe").unwrap_err();
        assert!(err.message.contains("true/false"));
    }

    #[test]
    fn unmatched_closing_paren_is_a_parse_error() {
        let err = parse("term)").unwrap_err();
        assert!(err.message.contains("unmatched"));
    }
}
