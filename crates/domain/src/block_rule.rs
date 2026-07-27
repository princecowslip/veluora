use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{BlockRuleId, ItemId};
use crate::media_item::MediaItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    ExactItem,
    Source,
    Creator,
    Series,
    Tag,
    Domain,
    FileHash,
    PerceptualHash,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    All,
    Local,
    External,
    SelectedSources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRule {
    pub id: BlockRuleId,
    pub rule_type: RuleType,
    /// Interpretation depends on `rule_type`: an item id for `ExactItem`, a
    /// tag's normalized value for `Tag`, a domain string for `Domain`, etc.
    pub target: String,
    pub scope: Scope,
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub enabled: bool,
}

/// The minimal facts a [`BlockRule`] needs to evaluate against, kept
/// separate from [`MediaItem`] so evaluation never depends on presentation
/// state (per Workstream 2's acceptance criteria in
/// `docs/46-implementation-plan.md`).
#[derive(Debug, Clone)]
pub struct BlockCandidate {
    pub item_id: ItemId,
    pub tag_values: Vec<String>,
    pub creator_ids: Vec<String>,
    pub source_id: Option<String>,
}

impl BlockCandidate {
    pub fn from_item(item: &MediaItem) -> Self {
        Self {
            item_id: item.id,
            tag_values: Vec::new(),
            creator_ids: item.creator_ids.iter().map(|id| id.to_string()).collect(),
            source_id: None,
        }
    }
}

impl BlockRule {
    /// Pure evaluation: does this rule match the given candidate?
    ///
    /// Deliberately has no dependency on GUI/CLI/TUI presentation code —
    /// application services call this directly.
    pub fn evaluate(&self, candidate: &BlockCandidate) -> bool {
        if !self.enabled {
            return false;
        }
        match self.rule_type {
            RuleType::ExactItem => candidate.item_id.to_string() == self.target,
            RuleType::Tag => candidate.tag_values.iter().any(|t| t == &self.target),
            RuleType::Creator => candidate.creator_ids.iter().any(|c| c == &self.target),
            RuleType::Source => candidate.source_id.as_deref() == Some(self.target.as_str()),
            // Domain/FileHash/PerceptualHash/Series/Query evaluation requires
            // data (connector metadata, hashes, saved query ASTs) that
            // doesn't exist yet in Milestone A; treat as non-matching until
            // the relevant milestone lands rather than guessing.
            RuleType::Domain
            | RuleType::FileHash
            | RuleType::PerceptualHash
            | RuleType::Series
            | RuleType::Query => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(rule_type: RuleType, target: &str, enabled: bool) -> BlockRule {
        BlockRule {
            id: BlockRuleId::new(),
            rule_type,
            target: target.to_string(),
            scope: Scope::All,
            reason: None,
            created_at: OffsetDateTime::now_utc(),
            enabled,
        }
    }

    #[test]
    fn tag_rule_matches_present_tag() {
        let r = rule(RuleType::Tag, "blocked-tag", true);
        let candidate = BlockCandidate {
            item_id: ItemId::new(),
            tag_values: vec!["blocked-tag".into()],
            creator_ids: vec![],
            source_id: None,
        };
        assert!(r.evaluate(&candidate));
    }

    #[test]
    fn disabled_rule_never_matches() {
        let r = rule(RuleType::Tag, "blocked-tag", false);
        let candidate = BlockCandidate {
            item_id: ItemId::new(),
            tag_values: vec!["blocked-tag".into()],
            creator_ids: vec![],
            source_id: None,
        };
        assert!(!r.evaluate(&candidate));
    }

    #[test]
    fn tag_rule_does_not_match_absent_tag() {
        let r = rule(RuleType::Tag, "blocked-tag", true);
        let candidate = BlockCandidate {
            item_id: ItemId::new(),
            tag_values: vec!["unrelated".into()],
            creator_ids: vec![],
            source_id: None,
        };
        assert!(!r.evaluate(&candidate));
    }
}
