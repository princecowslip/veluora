//! Block Rules: manage content-safety block rules (add, enable/disable,
//! remove) — the GUI counterpart of `veloura block-rule ...` and
//! `/api/v1/block-rules`, both of which predate this screen (see
//! `KNOWN_ISSUES.md`).

use std::sync::Arc;

use application::{AppContext, BlockRuleService};
use domain::{BlockRule, BlockRuleId, RuleType, Scope};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Element, Task};

const RULE_TYPES: [RuleType; 9] = [
    RuleType::ExactItem,
    RuleType::Source,
    RuleType::Creator,
    RuleType::Series,
    RuleType::Tag,
    RuleType::Domain,
    RuleType::FileHash,
    RuleType::PerceptualHash,
    RuleType::Query,
];

const SCOPES: [Scope; 4] = [
    Scope::All,
    Scope::Local,
    Scope::External,
    Scope::SelectedSources,
];

pub struct State {
    pub block_rules: Vec<BlockRule>,
    pub message: Option<String>,

    pub adding: bool,
    pub new_rule_type: RuleType,
    pub new_target: String,
    pub new_scope: Scope,
    pub new_reason: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            block_rules: Vec::new(),
            message: None,
            adding: false,
            new_rule_type: RuleType::ExactItem,
            new_target: String::new(),
            new_scope: Scope::All,
            new_reason: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    StartAdding,
    CancelAdding,
    RuleTypeChanged(RuleType),
    TargetChanged(String),
    ScopeChanged(Scope),
    ReasonChanged(String),
    ConfirmAdd,

    Enable(BlockRuleId),
    Disable(BlockRuleId),
    Remove(BlockRuleId),
}

pub fn refresh(state: &mut State, ctx: &Arc<AppContext>) {
    state.block_rules = BlockRuleService::list(ctx).unwrap_or_default();
}

pub fn update(state: &mut State, ctx: &Arc<AppContext>, message: Message) -> Task<Message> {
    match message {
        Message::StartAdding => {
            state.adding = true;
            state.new_rule_type = RuleType::ExactItem;
            state.new_target.clear();
            state.new_scope = Scope::All;
            state.new_reason.clear();
        }
        Message::CancelAdding => {
            state.adding = false;
        }
        Message::RuleTypeChanged(rule_type) => {
            state.new_rule_type = rule_type;
        }
        Message::TargetChanged(value) => {
            state.new_target = value;
        }
        Message::ScopeChanged(scope) => {
            state.new_scope = scope;
        }
        Message::ReasonChanged(value) => {
            state.new_reason = value;
        }
        Message::ConfirmAdd => {
            let reason = state.new_reason.trim();
            match BlockRuleService::create(
                ctx,
                state.new_rule_type,
                state.new_target.trim().to_string(),
                state.new_scope,
                if reason.is_empty() {
                    None
                } else {
                    Some(reason.to_string())
                },
            ) {
                Ok(_) => {
                    state.message = Some("Block rule added.".to_string());
                    state.adding = false;
                }
                Err(e) => state.message = Some(format!("Could not add block rule: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Enable(id) => {
            match BlockRuleService::set_enabled(ctx, id, true) {
                Ok(()) => state.message = Some("Block rule enabled.".to_string()),
                Err(e) => state.message = Some(format!("Could not enable block rule: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Disable(id) => {
            match BlockRuleService::set_enabled(ctx, id, false) {
                Ok(()) => state.message = Some("Block rule disabled.".to_string()),
                Err(e) => state.message = Some(format!("Could not disable block rule: {e}")),
            }
            refresh(state, ctx);
        }
        Message::Remove(id) => {
            match BlockRuleService::remove(ctx, id) {
                Ok(()) => state.message = Some("Block rule removed.".to_string()),
                Err(e) => state.message = Some(format!("Could not remove block rule: {e}")),
            }
            refresh(state, ctx);
        }
    }
    Task::none()
}

fn rule_row(rule: &BlockRule) -> Element<'_, Message> {
    row![
        text(rule.rule_type.to_string()).width(iced::Length::FillPortion(2)),
        text(rule.target.clone()).width(iced::Length::FillPortion(2)),
        text(rule.scope.to_string()).width(iced::Length::FillPortion(2)),
        text(rule.reason.clone().unwrap_or_default()).width(iced::Length::FillPortion(2)),
        checkbox("Enabled", rule.enabled).on_toggle(move |enabled| if enabled {
            Message::Enable(rule.id)
        } else {
            Message::Disable(rule.id)
        }),
        button("Remove").on_press(Message::Remove(rule.id)),
    ]
    .spacing(8)
    .into()
}

fn add_form(state: &State) -> Element<'_, Message> {
    column![
        text("Add a block rule").size(18),
        pick_list(
            RULE_TYPES,
            Some(state.new_rule_type),
            Message::RuleTypeChanged,
        ),
        text_input("Target", &state.new_target).on_input(Message::TargetChanged),
        pick_list(SCOPES, Some(state.new_scope), Message::ScopeChanged),
        text_input("Reason (optional)", &state.new_reason).on_input(Message::ReasonChanged),
        row![
            button("Add").on_press(Message::ConfirmAdd),
            button("Cancel").on_press(Message::CancelAdding),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![text("Block Rules").size(24)].spacing(12);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()));
    }

    if state.adding {
        content = content.push(add_form(state));
    } else {
        content = content.push(button("Add rule...").on_press(Message::StartAdding));
    }

    if state.block_rules.is_empty() {
        content = content.push(text("No block rules configured yet."));
    }
    let mut rules = column![].spacing(8);
    for rule in &state.block_rules {
        rules = rules.push(rule_row(rule));
    }
    content = content.push(scrollable(rules));

    container(content).padding(24).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> (Arc<AppContext>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Arc::new(AppContext::open_at(dir.path()).unwrap());
        (ctx, dir)
    }

    #[test]
    fn confirm_add_creates_a_rule_with_the_chosen_type_and_scope() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_rule_type: RuleType::Tag,
            new_target: "blocked-tag".to_string(),
            new_scope: Scope::External,
            new_reason: "spam".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert!(!state.adding);
        assert_eq!(state.block_rules.len(), 1);
        assert_eq!(state.block_rules[0].rule_type, RuleType::Tag);
        assert_eq!(state.block_rules[0].target, "blocked-tag");
        assert_eq!(state.block_rules[0].scope, Scope::External);
        assert_eq!(state.block_rules[0].reason.as_deref(), Some("spam"));
        assert!(state.block_rules[0].enabled);
    }

    #[test]
    fn confirm_add_omits_an_empty_reason() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_target: "some-item-id".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert!(state.block_rules[0].reason.is_none());
    }

    #[test]
    fn confirm_add_trims_whitespace_from_target_and_reason() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            adding: true,
            new_target: "  spaced-target  ".to_string(),
            new_reason: "  spaced reason  ".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::ConfirmAdd);

        assert_eq!(state.block_rules[0].target, "spaced-target");
        assert_eq!(
            state.block_rules[0].reason.as_deref(),
            Some("spaced reason")
        );
    }

    #[test]
    fn enable_and_disable_toggle_the_rule() {
        let (ctx, _dir) = test_ctx();
        let rule = BlockRuleService::create(
            &ctx,
            RuleType::ExactItem,
            "item-1".to_string(),
            Scope::All,
            None,
        )
        .unwrap();
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(&mut state, &ctx, Message::Disable(rule.id));
        assert!(!state.block_rules[0].enabled);

        let _ = update(&mut state, &ctx, Message::Enable(rule.id));
        assert!(state.block_rules[0].enabled);
    }

    #[test]
    fn remove_deletes_the_rule() {
        let (ctx, _dir) = test_ctx();
        let rule = BlockRuleService::create(
            &ctx,
            RuleType::ExactItem,
            "item-1".to_string(),
            Scope::All,
            None,
        )
        .unwrap();
        let mut state = State::default();
        refresh(&mut state, &ctx);

        let _ = update(&mut state, &ctx, Message::Remove(rule.id));

        assert!(state.block_rules.is_empty());
    }

    #[test]
    fn start_adding_resets_the_form_fields() {
        let (ctx, _dir) = test_ctx();
        let mut state = State {
            new_rule_type: RuleType::Domain,
            new_target: "stale".to_string(),
            new_scope: Scope::SelectedSources,
            new_reason: "stale reason".to_string(),
            ..State::default()
        };

        let _ = update(&mut state, &ctx, Message::StartAdding);

        assert!(state.adding);
        assert_eq!(state.new_rule_type, RuleType::ExactItem);
        assert!(state.new_target.is_empty());
        assert_eq!(state.new_scope, Scope::All);
        assert!(state.new_reason.is_empty());
    }
}
