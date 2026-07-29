#pragma once

#include <string>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// `GET /diagnostics/summary` — already implemented server-side, zero
// new backend work needed. Also shows the detected notcurses
// capability tier (Tier B/C — see `capability.h`), which
// `docs/09-terminal-ui.md` requires Diagnostics to surface.
class DiagnosticsView : public View {
 public:
  const char* title() const override { return "Diagnostics"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

  void set_capability_tier_label(std::string label) { capability_tier_label_ = std::move(label); }

 private:
  nlohmann::json summary_;
  std::string capability_tier_label_;
};

}  // namespace veloura
