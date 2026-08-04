//! CRUD for `domain::BlockRule` — `docs/21-content-safety-and-compliance.md`.
//!
//! The `block_rules` table and `BlockRule::evaluate` existed since
//! Milestone A/B, but the only code that ever touched the table was
//! `DownloadService::is_blocked`'s read-only query — there was no way
//! to create, list, or remove a rule anywhere (see `KNOWN_ISSUES.md`).
//! This is that surface; `DownloadService::is_blocked` keeps its own
//! query but reuses this module's row-mapping helpers instead of a
//! second copy.

use rusqlite::{params, Row};
use time::OffsetDateTime;

use domain::{BlockRule, BlockRuleId, RuleType, Scope};

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::time_format::{from_rfc3339, to_rfc3339};

pub struct BlockRuleService;

impl BlockRuleService {
    pub fn create(
        ctx: &AppContext,
        rule_type: RuleType,
        target: String,
        scope: Scope,
        reason: Option<String>,
    ) -> Result<BlockRule> {
        let rule = BlockRule {
            id: BlockRuleId::new(),
            rule_type,
            target,
            scope,
            reason,
            created_at: OffsetDateTime::now_utc(),
            enabled: true,
        };
        ctx.db
            .connection()
            .execute(
                "INSERT INTO block_rules (id, rule_type, target, scope, reason, created_at, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    rule.id.to_string(),
                    rule_type_to_str(rule.rule_type),
                    rule.target,
                    scope_to_str(rule.scope),
                    rule.reason,
                    to_rfc3339(rule.created_at),
                    rule.enabled as i64,
                ],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(rule)
    }

    pub fn list(ctx: &AppContext) -> Result<Vec<BlockRule>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_type, target, scope, reason, created_at, enabled \
                 FROM block_rules ORDER BY created_at",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map([], row_to_block_rule)
            .map_err(database::DatabaseError::from)?;
        let mut rules = Vec::new();
        for row in rows {
            rules.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(rules)
    }

    pub fn set_enabled(ctx: &AppContext, id: BlockRuleId, enabled: bool) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute(
                "UPDATE block_rules SET enabled = ?1 WHERE id = ?2",
                params![enabled as i64, id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("block rule {id}")));
        }
        Ok(())
    }

    pub fn remove(ctx: &AppContext, id: BlockRuleId) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute(
                "DELETE FROM block_rules WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("block rule {id}")));
        }
        Ok(())
    }
}

pub(crate) fn row_to_block_rule(row: &Row) -> rusqlite::Result<BlockRule> {
    let id: String = row.get(0)?;
    let rule_type: String = row.get(1)?;
    let scope: String = row.get(3)?;
    let created_at: String = row.get(5)?;
    Ok(BlockRule {
        id: BlockRuleId(parse_uuid(&id)?),
        rule_type: rule_type_from_str(&rule_type),
        target: row.get(2)?,
        scope: scope_from_str(&scope),
        reason: row.get(4)?,
        created_at: from_rfc3339(&created_at).unwrap_or(OffsetDateTime::UNIX_EPOCH),
        enabled: row.get::<_, i64>(6)? != 0,
    })
}

fn parse_uuid(s: &str) -> rusqlite::Result<uuid::Uuid> {
    uuid::Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn rule_type_from_str(s: &str) -> RuleType {
    use RuleType::*;
    match s {
        "exact_item" => ExactItem,
        "source" => Source,
        "creator" => Creator,
        "series" => Series,
        "tag" => Tag,
        "domain" => Domain,
        "file_hash" => FileHash,
        "perceptual_hash" => PerceptualHash,
        // `Query` always evaluates to non-matching (see
        // `BlockRule::evaluate`), the safest fallback for a rule type
        // this build doesn't recognize.
        _ => Query,
    }
}

pub(crate) fn rule_type_to_str(rule_type: RuleType) -> &'static str {
    match rule_type {
        RuleType::ExactItem => "exact_item",
        RuleType::Source => "source",
        RuleType::Creator => "creator",
        RuleType::Series => "series",
        RuleType::Tag => "tag",
        RuleType::Domain => "domain",
        RuleType::FileHash => "file_hash",
        RuleType::PerceptualHash => "perceptual_hash",
        RuleType::Query => "query",
    }
}

pub(crate) fn scope_from_str(s: &str) -> Scope {
    use Scope::*;
    match s {
        "local" => Local,
        "external" => External,
        "selected_sources" => SelectedSources,
        _ => All,
    }
}

pub(crate) fn scope_to_str(scope: Scope) -> &'static str {
    match scope {
        Scope::All => "all",
        Scope::Local => "local",
        Scope::External => "external",
        Scope::SelectedSources => "selected_sources",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AppContext;

    fn test_ctx() -> AppContext {
        AppContext::open_in_memory().unwrap()
    }

    #[test]
    fn create_list_round_trip() {
        let ctx = test_ctx();
        let rule = BlockRuleService::create(
            &ctx,
            RuleType::Tag,
            "blocked-tag".to_string(),
            Scope::All,
            Some("test reason".to_string()),
        )
        .unwrap();
        assert!(rule.enabled);

        let rules = BlockRuleService::list(&ctx).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule.id);
        assert_eq!(rules[0].rule_type, RuleType::Tag);
        assert_eq!(rules[0].target, "blocked-tag");
        assert_eq!(rules[0].reason.as_deref(), Some("test reason"));
    }

    #[test]
    fn set_enabled_toggles_flag() {
        let ctx = test_ctx();
        let rule =
            BlockRuleService::create(&ctx, RuleType::Source, "src".to_string(), Scope::All, None)
                .unwrap();
        BlockRuleService::set_enabled(&ctx, rule.id, false).unwrap();
        let rules = BlockRuleService::list(&ctx).unwrap();
        assert!(!rules[0].enabled);
    }

    #[test]
    fn set_enabled_missing_id_is_not_found() {
        let ctx = test_ctx();
        let err = BlockRuleService::set_enabled(&ctx, BlockRuleId::new(), true).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn remove_deletes_rule() {
        let ctx = test_ctx();
        let rule = BlockRuleService::create(&ctx, RuleType::Tag, "t".to_string(), Scope::All, None)
            .unwrap();
        BlockRuleService::remove(&ctx, rule.id).unwrap();
        assert!(BlockRuleService::list(&ctx).unwrap().is_empty());
    }

    #[test]
    fn remove_missing_id_is_not_found() {
        let ctx = test_ctx();
        let err = BlockRuleService::remove(&ctx, BlockRuleId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
