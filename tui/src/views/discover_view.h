#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Unified cross-source search: the local library plus every enabled
// connector-backed source, in one query — via `POST /api/v1/discover`.
// The TUI counterpart of the GUI Discover screen and `veloura discover
// ...`. Mirrors `LibraryView`'s edit-in-place search field rather than
// `SourcesView`'s separate browse mode, since Discover *is* a search
// view, not a source-management one — see `sources_view.h`'s header
// comment, which deferred this to exactly this view.
class DiscoverView : public View {
 public:
  const char* title() const override { return "Discover"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  void run_discover(ApiClient& api);

  std::string query_;
  bool editing_query_ = false;
  std::vector<nlohmann::json> hits_;
  std::vector<nlohmann::json> source_statuses_;
  int selected_row_ = 0;
};

}  // namespace veloura
