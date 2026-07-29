#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// `GET /items/:id` plus favorite/pin toggles, open (branches on the
// returned `OpenTarget`, launching an external player for video/audio
// via a direct argv-array spawn — never a shell), and an "add to
// collection" picker.
//
// Two things the GUI's Viewer has are intentionally not built here:
// notes editing (needs the same session-key flow the GUI has, which
// the TUI has no equivalent of this milestone) and clear-history/
// delete — `local-api` has no HTTP route for either (the GUI and CLI
// both call `UserStateService`/`ItemService` in-process instead).
// Adding those routes is out of Part 1's already-committed scope for
// this milestone, not something the TUI itself is missing.
class ItemDetailView : public View {
 public:
  const char* title() const override { return "Item"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

  // Loads a specific item — called by the app on navigation, not part
  // of the uniform `View` interface since it takes an id.
  void load(ApiClient& api, std::string item_id);

  bool has_item() const { return !item_id_.empty(); }

 private:
  void open_target(ApiClient& api);
  void toggle_favorite(ApiClient& api);
  void toggle_pin(ApiClient& api);
  void load_collections_for_picker(ApiClient& api);

  std::string item_id_;
  nlohmann::json detail_;
  nlohmann::json open_target_;
  bool has_open_target_ = false;
  std::string open_error_;

  bool picking_collection_ = false;
  std::vector<nlohmann::json> collections_for_picker_;
  int picker_selected_row_ = 0;
};

}  // namespace veloura
