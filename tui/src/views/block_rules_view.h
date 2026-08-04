#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Content-safety block rules: list, add, enable/disable, and remove a
// `domain::BlockRule` via `/api/v1/block-rules*`. The TUI counterpart of
// the GUI Block Rules screen and `veloura block-rule ...`, both of which
// predate this view (see `KNOWN_ISSUES.md`).
class BlockRulesView : public View {
 public:
  const char* title() const override { return "Block Rules"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  enum class AddStep { RuleType, Target, Scope, Reason };

  void render_list(ncplane* plane, unsigned rows, unsigned cols);
  void render_add_form(ncplane* plane, unsigned rows, unsigned cols);

  KeyOutcome handle_list_key(const ncinput& input, ApiClient& api);
  KeyOutcome handle_add_form_key(const ncinput& input, ApiClient& api);

  void reset_add_form();

  std::vector<nlohmann::json> rules_;
  int selected_row_ = 0;
  bool delete_confirm_armed_ = false;

  bool adding_ = false;
  AddStep add_step_ = AddStep::RuleType;
  std::string add_rule_type_;   // snake_case wire value, e.g. "exact_item"
  std::string add_target_input_;
  std::string add_scope_;       // snake_case wire value, e.g. "all"
  std::string add_reason_input_;
};

}  // namespace veloura
