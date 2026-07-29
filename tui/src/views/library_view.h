#pragma once

#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Search box + a single row-per-item list (`POST /search`). The full
// Detailed/Compact/Table/Two-column/Reel layout selector from
// `docs/09-terminal-ui.md` is deferred — one clean list view this
// milestone.
class LibraryView : public View {
 public:
  const char* title() const override { return "Library"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  void run_search(ApiClient& api);

  std::string query_;
  bool editing_query_ = false;
  std::vector<nlohmann::json> results_;
  long total_ = 0;
  int selected_row_ = 0;
};

}  // namespace veloura
