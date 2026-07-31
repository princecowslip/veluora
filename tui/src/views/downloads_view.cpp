#include "downloads_view.h"

#include "render_helpers.h"

namespace veloura {

void DownloadsView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/downloads");
  downloads_.clear();
  if (response.ok() && response.body.is_array()) {
    for (const auto& d : response.body) downloads_.push_back(d);
  }
  if (selected_row_ >= static_cast<int>(downloads_.size())) selected_row_ = 0;
}

void DownloadsView::render(ncplane* plane, unsigned rows, unsigned cols) {
  print_plain(plane, 0, 0,
              "Downloads — p: pause  r: resume  c: cancel  P: toggle pin  x: remove");
  if (downloads_.empty()) {
    print_plain(plane, 2, 0, "(no downloads yet — queue one from Item Detail)");
    return;
  }

  for (std::size_t i = 0; i < downloads_.size() && 2 + i < rows; ++i) {
    const auto& d = downloads_[i];
    const std::string title = d.value("item_title", "(unknown item)");
    const std::string source = d.value("source_display_name", "");
    const std::string state = d.value("state", "queued");
    const std::uint64_t received = d.value("bytes_received", 0ULL);

    std::string progress;
    if (d.contains("bytes_total") && !d["bytes_total"].is_null()) {
      const std::uint64_t total = d["bytes_total"].get<std::uint64_t>();
      progress = std::to_string(static_cast<long>(bytes_to_mb(received))) + "/" +
                 std::to_string(static_cast<long>(bytes_to_mb(total))) + " MB";
    } else {
      progress = std::to_string(static_cast<long>(bytes_to_mb(received))) + " MB";
    }

    std::string label = d.value("pinned", false) ? "* " : "  ";
    label += title;
    if (!source.empty()) label += "  (" + source + ")";
    label += "  [" + state + "]  " + progress;

    print_row(plane, static_cast<int>(2 + i), cols, label, selected_row_ == static_cast<int>(i));
  }
}

KeyOutcome DownloadsView::handle_key(const ncinput& input, ApiClient& api) {
  if (downloads_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(downloads_.size());
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ = (selected_row_ - 1 + static_cast<int>(downloads_.size())) %
                     static_cast<int>(downloads_.size());
    return KeyOutcome::handled();
  }

  const auto& selected = downloads_[static_cast<std::size_t>(selected_row_)];
  const std::string id = selected.value("id", "");

  if (input.id == 'p') {
    auto response = api.post("/api/v1/downloads/" + id + "/pause");
    status_message = response.ok() ? "Paused." : "Could not pause.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'r') {
    auto response = api.post("/api/v1/downloads/" + id + "/resume");
    status_message = response.ok() ? "Resuming." : "Could not resume.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'c') {
    auto response = api.post("/api/v1/downloads/" + id + "/cancel");
    status_message = response.ok() ? "Canceled." : "Could not cancel.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'P') {
    const bool currently_pinned = selected.value("pinned", false);
    auto response =
        api.post("/api/v1/downloads/" + id + "/pin", {{"pinned", !currently_pinned}});
    status_message =
        response.ok() ? (currently_pinned ? "Unpinned." : "Pinned.") : "Could not update pin.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'x') {
    // Preserves the library reference by default — matches the GUI's
    // and CLI's own safe default for this action.
    auto response = api.del("/api/v1/downloads/" + id + "?delete_file=false");
    status_message = response.ok() ? "Removed." : "Could not remove.";
    refresh(api);
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
