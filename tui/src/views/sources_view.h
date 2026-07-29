#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Configured connector-backed sources: list, add (local filesystem or
// RSS/Atom feed — the only two connectors that exist), enable/disable,
// remove, health-check, browse, and import a browsed item into the
// library — via `/api/v1/sources*`. The TUI counterpart of the GUI
// Sources screen and `veloura source ...`. There is no unified
// cross-source "Discover" backed by a single endpoint yet (browsing a
// source is a separate, explicit per-source action), so that stays out
// of scope here — see the tui-sources-view plan for the reasoning.
class SourcesView : public View {
 public:
  const char* title() const override { return "Sources"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  enum class AddStep { ChooseConnector, FeedUrl, DisplayName };

  void render_list(ncplane* plane, unsigned rows, unsigned cols);
  void render_add_form(ncplane* plane, unsigned rows, unsigned cols);
  void render_browse(ncplane* plane, unsigned rows, unsigned cols);

  KeyOutcome handle_list_key(const ncinput& input, ApiClient& api);
  KeyOutcome handle_add_form_key(const ncinput& input, ApiClient& api);
  KeyOutcome handle_browse_key(const ncinput& input, ApiClient& api);

  void reset_add_form();

  std::vector<nlohmann::json> sources_;
  int selected_row_ = 0;
  bool delete_confirm_armed_ = false;

  bool adding_ = false;
  AddStep add_step_ = AddStep::ChooseConnector;
  std::string add_connector_id_;
  std::string add_feed_url_input_;
  std::string add_display_name_input_;

  bool browsing_ = false;
  std::string browse_source_id_;
  std::string browse_source_name_;
  std::vector<nlohmann::json> browse_results_;
  int browse_selected_row_ = 0;
};

}  // namespace veloura
