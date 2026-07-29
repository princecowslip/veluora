#include "diagnostics_view.h"

#include "render_helpers.h"

namespace veloura {

void DiagnosticsView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/diagnostics/summary");
  if (response.ok()) summary_ = response.body;
}

void DiagnosticsView::render(ncplane* plane, unsigned rows, unsigned cols) {
  (void)cols;
  int y = 0;
  print_plain(plane, y++, 0, "Diagnostics");
  ++y;
  print_plain(plane, y++, 0, "TUI capability tier: " + capability_tier_label_);
  ++y;
  if (!summary_.is_object()) {
    print_plain(plane, y++, 0, "(could not load diagnostics summary)");
    return;
  }
  print_plain(plane, y++, 0, "Data dir:            " + summary_.value("data_dir", ""));
  print_plain(plane, y++, 0, "Database:            " + summary_.value("db_path", ""));
  print_plain(plane, y++, 0, "Applied migrations:  " + std::to_string(summary_.value("applied_migrations", 0)));
  if (static_cast<unsigned>(y) < rows) {
    print_plain(plane, y++, 0,
                std::string("ffprobe:             ") +
                    (summary_.value("ffprobe_available", false) ? "found" : "not found"));
  }
  if (static_cast<unsigned>(y) < rows) {
    print_plain(plane, y++, 0,
                std::string("ffmpeg:              ") +
                    (summary_.value("ffmpeg_available", false) ? "found" : "not found"));
  }
}

KeyOutcome DiagnosticsView::handle_key(const ncinput&, ApiClient&) { return KeyOutcome::unhandled(); }

}  // namespace veloura
