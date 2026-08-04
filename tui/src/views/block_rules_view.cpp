#include "block_rules_view.h"

#include "render_helpers.h"

namespace veloura {

namespace {

bool is_text_char(std::uint32_t id) { return id >= 0x20 && id < 0x7f; }

// `domain::RuleType`/`domain::Scope` (see `crates/domain/src/block_rule.rs`)
// serialize as these snake_case strings. Only ExactItem/Tag/Creator/Source
// actually match anything in `BlockRule::evaluate` today.
bool rule_type_is_enforced(const std::string& rule_type) {
  return rule_type == "exact_item" || rule_type == "source" || rule_type == "creator" ||
         rule_type == "tag";
}

}  // namespace

void BlockRulesView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/block-rules");
  rules_.clear();
  if (response.ok() && response.body.is_array()) {
    for (const auto& r : response.body) rules_.push_back(r);
  }
  if (selected_row_ >= static_cast<int>(rules_.size())) selected_row_ = 0;
}

void BlockRulesView::reset_add_form() {
  adding_ = false;
  add_step_ = AddStep::RuleType;
  add_rule_type_.clear();
  add_target_input_.clear();
  add_scope_.clear();
  add_reason_input_.clear();
}

void BlockRulesView::render(ncplane* plane, unsigned rows, unsigned cols) {
  if (adding_) {
    render_add_form(plane, rows, cols);
  } else {
    render_list(plane, rows, cols);
  }
}

void BlockRulesView::render_list(ncplane* plane, unsigned rows, unsigned cols) {
  print_plain(plane, 0, 0, "Block Rules — a: add   e: enable/disable   x: remove");
  if (rules_.empty()) {
    print_plain(plane, 2, 0, "(no block rules configured yet — press a to add one)");
    return;
  }
  for (std::size_t i = 0; i < rules_.size() && 2 + i < rows; ++i) {
    const auto& r = rules_[i];
    const std::string rule_type = r.value("rule_type", "");
    std::string label = rule_type;
    label += "  target=" + r.value("target", "");
    label += "  scope=" + r.value("scope", "");
    label += r.value("enabled", false) ? "  enabled" : "  disabled";
    if (r.contains("reason") && r["reason"].is_string()) {
      label += "  reason=" + r["reason"].get<std::string>();
    }
    if (!rule_type_is_enforced(rule_type)) {
      label += "  [not enforced yet]";
    }
    print_row(plane, static_cast<int>(2 + i), cols, label, selected_row_ == static_cast<int>(i));
  }

  if (delete_confirm_armed_ && !rules_.empty()) {
    const unsigned confirm_row = 3 + static_cast<unsigned>(rules_.size());
    if (confirm_row < rows) {
      print_plain(plane, static_cast<int>(confirm_row), 0,
                  "Remove this block rule? Press x again to confirm, Esc to cancel.");
    }
  }
}

void BlockRulesView::render_add_form(ncplane* plane, unsigned rows, unsigned cols) {
  (void)cols;
  (void)rows;
  print_plain(plane, 0, 0, "Add block rule");
  switch (add_step_) {
    case AddStep::RuleType:
      print_plain(plane, 2, 0,
                  "e: exact item  s: source  c: creator  r: series  t: tag  d: domain  f: file hash  "
                  "p: perceptual hash  q: query   Esc: cancel");
      break;
    case AddStep::Target:
      print_plain(plane, 2, 0, "Target: " + add_target_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::Scope:
      print_plain(plane, 2, 0,
                  "a: all  l: local only  x: external only  s: selected sources   Esc: cancel");
      break;
    case AddStep::Reason:
      print_plain(plane, 2, 0, "Reason (optional): " + add_reason_input_ + "_");
      print_plain(plane, 4, 0, "Enter to add, Esc to cancel");
      break;
  }
}

KeyOutcome BlockRulesView::handle_key(const ncinput& input, ApiClient& api) {
  if (adding_) return handle_add_form_key(input, api);
  return handle_list_key(input, api);
}

KeyOutcome BlockRulesView::handle_list_key(const ncinput& input, ApiClient& api) {
  if (input.id == 'a') {
    reset_add_form();
    adding_ = true;
    return KeyOutcome::handled();
  }

  if (rules_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(rules_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ =
        (selected_row_ - 1 + static_cast<int>(rules_.size())) % static_cast<int>(rules_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }

  const auto& selected = rules_[static_cast<std::size_t>(selected_row_)];
  const std::string id = selected.value("id", "");

  if (input.id == 'e') {
    const bool enabled = selected.value("enabled", false);
    auto response =
        api.post(std::string("/api/v1/block-rules/") + id + (enabled ? "/disable" : "/enable"));
    status_message = response.ok() ? (enabled ? "Block rule disabled." : "Block rule enabled.")
                                    : "Could not update block rule.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'x') {
    if (delete_confirm_armed_) {
      api.del("/api/v1/block-rules/" + id);
      delete_confirm_armed_ = false;
      refresh(api);
    } else {
      delete_confirm_armed_ = true;
    }
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_ESC && delete_confirm_armed_) {
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

KeyOutcome BlockRulesView::handle_add_form_key(const ncinput& input, ApiClient& api) {
  if (input.id == NCKEY_ESC) {
    reset_add_form();
    return KeyOutcome::handled();
  }

  switch (add_step_) {
    case AddStep::RuleType:
      if (input.id == 'e') {
        add_rule_type_ = "exact_item";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 's') {
        add_rule_type_ = "source";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'c') {
        add_rule_type_ = "creator";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'r') {
        add_rule_type_ = "series";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 't') {
        add_rule_type_ = "tag";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'd') {
        add_rule_type_ = "domain";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'f') {
        add_rule_type_ = "file_hash";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'p') {
        add_rule_type_ = "perceptual_hash";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      if (input.id == 'q') {
        add_rule_type_ = "query";
        add_step_ = AddStep::Target;
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::Target:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::Scope;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_target_input_.empty()) add_target_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_target_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::Scope:
      if (input.id == 'a') {
        add_scope_ = "all";
        add_step_ = AddStep::Reason;
        return KeyOutcome::handled();
      }
      if (input.id == 'l') {
        add_scope_ = "local";
        add_step_ = AddStep::Reason;
        return KeyOutcome::handled();
      }
      if (input.id == 'x') {
        add_scope_ = "external";
        add_step_ = AddStep::Reason;
        return KeyOutcome::handled();
      }
      if (input.id == 's') {
        add_scope_ = "selected_sources";
        add_step_ = AddStep::Reason;
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::Reason:
      if (input.id == NCKEY_ENTER) {
        nlohmann::json body = {{"rule_type", add_rule_type_},
                                {"target", add_target_input_},
                                {"scope", add_scope_}};
        if (!add_reason_input_.empty()) {
          body["reason"] = add_reason_input_;
        }
        auto response = api.post("/api/v1/block-rules", body);
        status_message = response.ok() ? "Block rule added." : "Could not add block rule.";
        reset_add_form();
        refresh(api);
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_reason_input_.empty()) add_reason_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_reason_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();
  }
  return KeyOutcome::handled();
}

}  // namespace veloura
