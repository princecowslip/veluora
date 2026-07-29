#pragma once

#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Continue list (`GET /home/continue`) and recently added items
// (`POST /search` with an empty query) — the two Home-view sections
// scoped for this milestone; source notices and download activity are
// skipped, there being no connectors yet.
class HomeView : public View {
 public:
  const char* title() const override { return "Home"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  std::vector<nlohmann::json> continue_items_;
  std::vector<nlohmann::json> recent_items_;
  // 0-based row across both sections combined (continue_items_ first,
  // then recent_items_), for a single unified up/down selection.
  int selected_row_ = 0;

  int total_rows() const;
  const nlohmann::json* item_at_row(int row) const;
};

}  // namespace veloura
