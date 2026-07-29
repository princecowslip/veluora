#include "home_view.h"

#include "render_helpers.h"

namespace veloura {

void HomeView::refresh(ApiClient& api) {
  continue_items_.clear();
  recent_items_.clear();
  selected_row_ = 0;

  auto continue_response = api.get("/api/v1/home/continue?limit=10");
  if (continue_response.ok() && continue_response.body.is_array()) {
    for (const auto& item : continue_response.body) {
      continue_items_.push_back(item);
    }
  }

  auto recent_response = api.post("/api/v1/search", {{"query", ""}, {"limit", 10}});
  if (recent_response.ok() && recent_response.body.contains("items")) {
    for (const auto& item : recent_response.body["items"]) {
      recent_items_.push_back(item);
    }
  }
}

int HomeView::total_rows() const {
  return static_cast<int>(continue_items_.size() + recent_items_.size());
}

const nlohmann::json* HomeView::item_at_row(int row) const {
  if (row < 0) return nullptr;
  if (static_cast<std::size_t>(row) < continue_items_.size()) {
    return &continue_items_[row];
  }
  const std::size_t recent_index = static_cast<std::size_t>(row) - continue_items_.size();
  if (recent_index < recent_items_.size()) {
    return &recent_items_[recent_index];
  }
  return nullptr;
}

void HomeView::render(ncplane* plane, unsigned rows, unsigned cols) {
  int y = 0;
  print_plain(plane, y++, 0, "Continue");
  if (continue_items_.empty()) {
    print_plain(plane, y++, 2, "(nothing in progress)");
  } else {
    for (std::size_t i = 0; i < continue_items_.size() && static_cast<unsigned>(y) < rows; ++i, ++y) {
      const auto& item = continue_items_[i];
      std::string label = "  " + item.value("title", "?") + "  [" +
                           item.value("media_type", "") + "]" +
                           (item.value("favorite", false) ? " *" : "");
      print_row(plane, y, cols, label, selected_row_ == static_cast<int>(i));
    }
  }

  if (static_cast<unsigned>(y) >= rows) return;
  ++y;
  if (static_cast<unsigned>(y) >= rows) return;
  print_plain(plane, y++, 0, "Recently added");
  if (recent_items_.empty()) {
    print_plain(plane, y++, 2, "(library is empty — add a folder in the CLI/GUI)");
  } else {
    for (std::size_t i = 0; i < recent_items_.size() && static_cast<unsigned>(y) < rows; ++i, ++y) {
      const auto& item = recent_items_[i];
      std::string label = "  " + item.value("title", "?") + "  [" +
                           item.value("media_type", "") + "]" +
                           (item.value("favorite", false) ? " *" : "");
      const int row_index = static_cast<int>(continue_items_.size() + i);
      print_row(plane, y, cols, label, selected_row_ == row_index);
    }
  }
}

KeyOutcome HomeView::handle_key(const ncinput& input, ApiClient&) {
  const int rows = total_rows();
  if (rows == 0) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % rows;
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ = (selected_row_ - 1 + rows) % rows;
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_ENTER || input.id == ' ') {
    const auto* item = item_at_row(selected_row_);
    if (item != nullptr && item->contains("item_id")) {
      return KeyOutcome::open(item->at("item_id").get<std::string>());
    }
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
