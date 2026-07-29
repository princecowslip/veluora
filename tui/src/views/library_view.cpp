#include "library_view.h"

#include "render_helpers.h"

namespace veloura {

void LibraryView::refresh(ApiClient& api) { run_search(api); }

void LibraryView::run_search(ApiClient& api) {
  auto response = api.post("/api/v1/search", {{"query", query_}, {"limit", 200}, {"offset", 0}});
  results_.clear();
  selected_row_ = 0;
  if (response.ok() && response.body.contains("items")) {
    for (const auto& item : response.body["items"]) {
      results_.push_back(item);
    }
    total_ = response.body.value("total", (long)results_.size());
    status_message.clear();
  } else if (!response.network_error && response.body.contains("error")) {
    status_message = "search error: " + response.body.value("error", "");
  } else {
    status_message.clear();
  }
}

void LibraryView::render(ncplane* plane, unsigned rows, unsigned cols) {
  std::string search_line = std::string("Search: ") + query_ + (editing_query_ ? "_" : "");
  print_plain(plane, 0, 0, search_line);
  print_plain(plane, 1, 0, "(" + std::to_string(total_) + " result(s) — press / to edit, Enter to open)");

  const int list_top = 3;
  for (std::size_t i = 0; i < results_.size() && list_top + static_cast<int>(i) < static_cast<int>(rows);
       ++i) {
    const auto& item = results_[i];
    std::string label = item.value("title", "?") + "  [" + item.value("media_type", "") + "]" +
                         (item.value("favorite", false) ? " *" : "");
    print_row(plane, list_top + static_cast<int>(i), cols, label, selected_row_ == static_cast<int>(i));
  }
}

KeyOutcome LibraryView::handle_key(const ncinput& input, ApiClient& api) {
  if (editing_query_) {
    if (input.id == NCKEY_ENTER) {
      editing_query_ = false;
      run_search(api);
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_ESC) {
      editing_query_ = false;
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_BACKSPACE || input.id == 127) {
      if (!query_.empty()) query_.pop_back();
      return KeyOutcome::handled();
    }
    if (input.id >= 0x20 && input.id < 0x7f) {
      query_.push_back(static_cast<char>(input.id));
      return KeyOutcome::handled();
    }
    // Swallow everything else while editing so global bindings never
    // fire mid-typing.
    return KeyOutcome::handled();
  }

  if (input.id == '/') {
    editing_query_ = true;
    return KeyOutcome::handled();
  }
  if (results_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(results_.size());
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ = (selected_row_ - 1 + static_cast<int>(results_.size())) % static_cast<int>(results_.size());
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_ENTER || input.id == ' ') {
    const auto& item = results_[static_cast<std::size_t>(selected_row_)];
    if (item.contains("item_id")) {
      return KeyOutcome::open(item.at("item_id").get<std::string>());
    }
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
