#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Downloads and offline use (Workstream 11) — the TUI counterpart of
// `veloura download ...` and `/api/v1/downloads`. Distinct from
// `CacheView` (F4 — the local-only pinned/quota reinterpretation from
// before any connector existed, over files that are already local):
// this is the real remote download queue Milestone F's connectors made
// possible, closing the "No Queue view exists yet" gap `KNOWN_ISSUES.md`
// flagged for the TUI.
class DownloadsView : public View {
 public:
  const char* title() const override { return "Downloads"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  std::vector<nlohmann::json> downloads_;
  int selected_row_ = 0;
};

}  // namespace veloura
