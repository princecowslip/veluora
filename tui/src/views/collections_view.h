#pragma once

#include <vector>

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Lists, creates, and deletes manual collections. Adding an item to a
// collection lives in `ItemDetailView` (where an item id is already in
// context) rather than here. Removing a single item from a collection
// isn't built: `local-api` has no route to list a collection's
// contents, so there's nothing to pick a removal target from — see the
// comment in `collections_view.cpp`.
class CollectionsView : public View {
 public:
  const char* title() const override { return "Collections"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  std::vector<nlohmann::json> collections_;
  int selected_row_ = 0;

  bool creating_ = false;
  std::string name_input_;

  bool delete_confirm_armed_ = false;
};

}  // namespace veloura
