#include "item_detail_view.h"

#include <unistd.h>

#include <cstdlib>

#include "render_helpers.h"

namespace veloura {

namespace {

// Spawns `argv[0]` with `argv`, never through a shell. The parent
// doesn't wait — `main()` sets `SIGCHLD` to `SIG_IGN` at startup so
// the kernel reaps the child itself once it exits, avoiding zombies
// without the TUI needing to track player pids.
void spawn_detached(const std::vector<std::string>& args) {
  if (args.empty()) return;
  pid_t pid = fork();
  if (pid == 0) {
    std::vector<char*> argv;
    argv.reserve(args.size() + 1);
    for (auto& a : args) argv.push_back(const_cast<char*>(a.c_str()));
    argv.push_back(nullptr);
    execvp(argv[0], argv.data());
    _exit(127);
  }
}

}  // namespace

void ItemDetailView::load(ApiClient& api, std::string item_id) {
  item_id_ = std::move(item_id);
  has_open_target_ = false;
  open_error_.clear();
  picking_collection_ = false;
  refresh(api);
}

void ItemDetailView::refresh(ApiClient& api) {
  if (item_id_.empty()) return;
  auto response = api.get("/api/v1/items/" + item_id_);
  if (response.ok()) {
    detail_ = response.body;
  }
}

void ItemDetailView::toggle_favorite(ApiClient& api) {
  const bool current = detail_.value("favorite", false);
  auto response = api.post("/api/v1/items/" + item_id_ + "/favorite", {{"favorite", !current}});
  if (response.ok()) {
    status_message = !current ? "Favorited." : "Unfavorited.";
  } else {
    status_message = "Could not update favorite.";
  }
  refresh(api);
}

void ItemDetailView::toggle_pin(ApiClient& api) {
  const bool current = detail_.value("pinned", false);
  auto response = api.post("/api/v1/items/" + item_id_ + "/pin", {{"pinned", !current}});
  if (response.ok()) {
    status_message = !current ? "Pinned." : "Unpinned.";
  } else {
    status_message = "Could not update pin.";
  }
  refresh(api);
}

void ItemDetailView::open_target(ApiClient& api) {
  auto response = api.post("/api/v1/items/" + item_id_ + "/open");
  if (!response.ok()) {
    has_open_target_ = false;
    open_error_ = "Could not resolve open target.";
    return;
  }
  open_target_ = response.body;
  has_open_target_ = true;
  open_error_.clear();

  const std::string kind = open_target_.value("kind", "");
  if (kind == "external_player") {
    const std::string local_path = open_target_.value("local_path", "");
    const char* player = std::getenv("VELOURA_TUI_PLAYER");
    const std::string player_bin = (player != nullptr && player[0] != '\0') ? player : "xdg-open";
    spawn_detached({player_bin, local_path});
    status_message = "Launched " + player_bin + " (set VELOURA_TUI_PLAYER to override).";
  }
}

void ItemDetailView::load_collections_for_picker(ApiClient& api) {
  auto response = api.get("/api/v1/collections");
  collections_for_picker_.clear();
  picker_selected_row_ = 0;
  if (response.ok() && response.body.is_array()) {
    for (const auto& c : response.body) collections_for_picker_.push_back(c);
  }
}

void ItemDetailView::render(ncplane* plane, unsigned rows, unsigned cols) {
  if (item_id_.empty()) {
    print_plain(plane, 0, 0, "No item selected.");
    return;
  }

  if (picking_collection_) {
    print_plain(plane, 0, 0, "Add to collection — Enter to add, Esc to cancel");
    if (collections_for_picker_.empty()) {
      print_plain(plane, 2, 0, "(no collections yet — create one from the Collections view)");
      return;
    }
    for (std::size_t i = 0; i < collections_for_picker_.size() && 2 + i < rows; ++i) {
      print_row(plane, static_cast<int>(2 + i), cols, collections_for_picker_[i].value("name", "?"),
                picker_selected_row_ == static_cast<int>(i));
    }
    return;
  }

  int y = 0;
  print_plain(plane, y++, 0, detail_.value("title", "?"));
  std::string flags = std::string("Type: ") + detail_.value("media_type", "") +
                       "   Favorite: " + (detail_.value("favorite", false) ? "yes" : "no") +
                       "   Pinned: " + (detail_.value("pinned", false) ? "yes" : "no");
  if (detail_.contains("rating") && !detail_["rating"].is_null()) {
    flags += "   Rating: " + std::to_string(detail_["rating"].get<int>());
  }
  print_plain(plane, y++, 0, flags);

  if (detail_.contains("tags") && !detail_["tags"].empty()) {
    std::string tags = "Tags: ";
    for (const auto& tag : detail_["tags"]) tags += tag.get<std::string>() + " ";
    print_plain(plane, y++, 0, tags);
  }

  ++y;
  print_plain(plane, y++, 0, "Variants:");
  if (detail_.contains("variants")) {
    for (const auto& variant : detail_["variants"]) {
      if (static_cast<unsigned>(y) >= rows) break;
      std::string line = "  " + variant.value("local_path", "<no local file>") + "  (" +
                          variant.value("mime_type", "") + ")";
      print_plain(plane, y++, 0, line);
    }
  }

  if (has_open_target_) {
    ++y;
    if (static_cast<unsigned>(y) < rows) {
      const std::string kind = open_target_.value("kind", "");
      std::string line = "Open target: " + kind;
      if (kind == "pages") {
        line += "  (" + std::to_string(open_target_.value("page_count", 0)) + " page(s))";
      } else if (kind == "story" && open_target_.contains("chapter_map")) {
        line += "  (" + std::to_string(open_target_["chapter_map"].size()) + " chapter(s))";
      }
      print_plain(plane, y++, 0, line);
    }
  } else if (!open_error_.empty() && static_cast<unsigned>(y) < rows) {
    ++y;
    print_plain(plane, y++, 0, open_error_);
  }
}

KeyOutcome ItemDetailView::handle_key(const ncinput& input, ApiClient& api) {
  if (item_id_.empty()) return KeyOutcome::unhandled();

  if (picking_collection_) {
    if (input.id == NCKEY_ESC) {
      picking_collection_ = false;
      return KeyOutcome::handled();
    }
    if (collections_for_picker_.empty()) return KeyOutcome::handled();
    if (input.id == NCKEY_DOWN || input.id == 'j') {
      picker_selected_row_ =
          (picker_selected_row_ + 1) % static_cast<int>(collections_for_picker_.size());
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_UP || input.id == 'k') {
      picker_selected_row_ = (picker_selected_row_ - 1 + static_cast<int>(collections_for_picker_.size())) %
                              static_cast<int>(collections_for_picker_.size());
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_ENTER) {
      const auto& collection = collections_for_picker_[static_cast<std::size_t>(picker_selected_row_)];
      auto response = api.post("/api/v1/collections/" + collection.value("id", "") + "/items",
                                {{"item_id", item_id_}});
      status_message = response.ok() ? "Added to " + collection.value("name", "collection") + "."
                                      : "Could not add to collection.";
      picking_collection_ = false;
      return KeyOutcome::handled();
    }
    return KeyOutcome::handled();
  }

  if (input.id == 'f') {
    toggle_favorite(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'p') {
    toggle_pin(api);
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_ENTER || input.id == 'o' || input.id == ' ') {
    open_target(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'c') {
    load_collections_for_picker(api);
    picking_collection_ = true;
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
