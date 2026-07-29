#include "collections_view.h"

#include "render_helpers.h"

namespace veloura {

void CollectionsView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/collections");
  collections_.clear();
  if (response.ok() && response.body.is_array()) {
    for (const auto& c : response.body) collections_.push_back(c);
  }
  if (selected_row_ >= static_cast<int>(collections_.size())) selected_row_ = 0;
}

void CollectionsView::render(ncplane* plane, unsigned rows, unsigned cols) {
  if (creating_) {
    print_plain(plane, 0, 0, "New collection name: " + name_input_ + "_");
    print_plain(plane, 2, 0, "Enter to create, Esc to cancel");
    return;
  }

  print_plain(plane, 0, 0, "Collections — n: new   x: delete   Enter: add current item (from Item Detail)");
  if (collections_.empty()) {
    print_plain(plane, 2, 0, "(no collections yet — press n to create one)");
    return;
  }
  for (std::size_t i = 0; i < collections_.size() && 2 + i < rows; ++i) {
    const auto& c = collections_[i];
    std::string label = c.value("name", "?");
    if (c.contains("description") && !c["description"].is_null()) {
      label += "  — " + c["description"].get<std::string>();
    }
    print_row(plane, static_cast<int>(2 + i), cols, label, selected_row_ == static_cast<int>(i));
  }

  if (delete_confirm_armed_ && !collections_.empty()) {
    const unsigned confirm_row = 3 + static_cast<unsigned>(collections_.size());
    if (confirm_row < rows) {
      print_plain(plane, static_cast<int>(confirm_row), 0,
                  "Delete '" + collections_[static_cast<std::size_t>(selected_row_)].value("name", "?") +
                      "'? Press x again to confirm, Esc to cancel.");
    }
  }
}

KeyOutcome CollectionsView::handle_key(const ncinput& input, ApiClient& api) {
  if (creating_) {
    if (input.id == NCKEY_ENTER) {
      if (!name_input_.empty()) {
        api.post("/api/v1/collections", {{"name", name_input_}});
        refresh(api);
      }
      creating_ = false;
      name_input_.clear();
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_ESC) {
      creating_ = false;
      name_input_.clear();
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_BACKSPACE || input.id == 127) {
      if (!name_input_.empty()) name_input_.pop_back();
      return KeyOutcome::handled();
    }
    if (input.id >= 0x20 && input.id < 0x7f) {
      name_input_.push_back(static_cast<char>(input.id));
      return KeyOutcome::handled();
    }
    return KeyOutcome::handled();
  }

  if (input.id == 'n') {
    creating_ = true;
    return KeyOutcome::handled();
  }

  if (collections_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(collections_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ = (selected_row_ - 1 + static_cast<int>(collections_.size())) %
                     static_cast<int>(collections_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  if (input.id == 'x') {
    if (delete_confirm_armed_) {
      const auto& c = collections_[static_cast<std::size_t>(selected_row_)];
      api.del("/api/v1/collections/" + c.value("id", ""));
      delete_confirm_armed_ = false;
      refresh(api);
    } else {
      delete_confirm_armed_ = true;
    }
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_ESC && delete_confirm_armed_) {
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
