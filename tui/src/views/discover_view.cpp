#include "discover_view.h"

#include "render_helpers.h"

namespace veloura {

void DiscoverView::refresh(ApiClient& api) { run_discover(api); }

void DiscoverView::run_discover(ApiClient& api) {
  auto response = api.post("/api/v1/discover", {{"query", query_}, {"limit_per_source", 25}});
  hits_.clear();
  source_statuses_.clear();
  selected_row_ = 0;
  if (response.ok() && response.body.contains("hits") && response.body["hits"].is_array()) {
    for (const auto& hit : response.body["hits"]) hits_.push_back(hit);
    if (response.body.contains("sources") && response.body["sources"].is_array()) {
      for (const auto& status : response.body["sources"]) source_statuses_.push_back(status);
    }
    status_message.clear();
  } else if (!response.network_error && response.body.contains("error")) {
    status_message = "discover error: " + response.body.value("error", "");
  } else {
    status_message.clear();
  }
}

void DiscoverView::render(ncplane* plane, unsigned rows, unsigned cols) {
  std::string search_line = std::string("Search: ") + query_ + (editing_query_ ? "_" : "");
  print_plain(plane, 0, 0, search_line);
  print_plain(plane, 1, 0,
              "(" + std::to_string(hits_.size()) + " hit(s) across " +
                  std::to_string(source_statuses_.size()) +
                  " source(s) — press / to edit, Enter to search, i to import)");

  unsigned row = 3;
  for (const auto& status : source_statuses_) {
    if (row >= rows) break;
    std::string source_status = "success";
    if (status.contains("status") && status["status"].contains("status")) {
      source_status = status["status"]["status"].get<std::string>();
    }
    const bool ok = source_status == "success" || source_status == "partial";
    const bool has_unsupported = status.contains("unsupported_clauses") &&
                                  status["unsupported_clauses"].is_array() &&
                                  !status["unsupported_clauses"].empty();
    if (ok && !has_unsupported) continue;
    std::string display_name = status.contains("source_display_name")
                                    ? status["source_display_name"].get<std::string>()
                                    : "?";
    std::string line = display_name + ": " + source_status;
    if (has_unsupported) line += " (unsupported query clauses)";
    print_plain(plane, static_cast<int>(row), 0, line);
    ++row;
  }

  for (std::size_t i = 0; i < hits_.size() && row + i < rows; ++i) {
    const auto& hit = hits_[i];
    std::string title = "(untitled)";
    if (hit.contains("item") && hit["item"].contains("title")) {
      title = hit["item"]["title"].get<std::string>();
    }
    std::string source_name = hit.contains("source_display_name")
                                   ? hit["source_display_name"].get<std::string>()
                                   : "?";
    std::string label = source_name + "  " + title;
    if (hit.contains("local_item_id") && !hit["local_item_id"].is_null()) {
      label += "  [in library]";
    }
    print_row(plane, static_cast<int>(row + i), cols, label, selected_row_ == static_cast<int>(i));
  }
}

KeyOutcome DiscoverView::handle_key(const ncinput& input, ApiClient& api) {
  if (editing_query_) {
    if (input.id == NCKEY_ENTER) {
      editing_query_ = false;
      run_discover(api);
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
    // fire mid-typing — matches `LibraryView`'s search-field handling.
    return KeyOutcome::handled();
  }

  if (input.id == '/') {
    editing_query_ = true;
    return KeyOutcome::handled();
  }
  if (hits_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(hits_.size());
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ =
        (selected_row_ - 1 + static_cast<int>(hits_.size())) % static_cast<int>(hits_.size());
    return KeyOutcome::handled();
  }
  if (input.id == 'i') {
    auto& hit = hits_[static_cast<std::size_t>(selected_row_)];
    if (hit.contains("local_item_id") && !hit["local_item_id"].is_null()) {
      status_message = "Already in the library.";
      return KeyOutcome::handled();
    }
    const std::string source_id = hit.contains("source_id") ? hit["source_id"].get<std::string>() : "";
    const nlohmann::json item = hit.contains("item") ? hit["item"] : nlohmann::json::object();
    auto response = api.post("/api/v1/sources/" + source_id + "/import", item);
    if (response.ok() && response.body.contains("item_id")) {
      status_message = "Imported into the library.";
      hit["local_item_id"] = response.body["item_id"];
    } else {
      status_message = "Could not import item.";
    }
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
