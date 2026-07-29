#include "privacy_view.h"

#include "render_helpers.h"

namespace veloura {

void PrivacyView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/privacy/status");
  if (response.ok()) status_ = response.body;
}

void PrivacyView::render(ncplane* plane, unsigned rows, unsigned cols) {
  (void)cols;
  int y = 0;
  print_plain(plane, y++, 0, "Privacy");
  ++y;
  if (!status_.is_object()) {
    print_plain(plane, y++, 0, "(could not load privacy status)");
    return;
  }
  print_plain(plane, y++, 0,
              std::string("Profile password: ") + (status_.value("has_password", false) ? "set" : "not set"));
  print_plain(plane, y++, 0, std::string("Metadata encryption: ") +
                                  (status_.value("metadata_encryption_enabled", false) ? "on" : "off"));
  ++y;
  if (static_cast<unsigned>(y) < rows) {
    print_plain(plane, y++, 0, "Ctrl+L locks the session.");
  }
  if (static_cast<unsigned>(y) < rows) {
    print_plain(plane, y++, 0, "Setting a password isn't available from the TUI yet — use the GUI or CLI.");
  }
}

KeyOutcome PrivacyView::handle_key(const ncinput&, ApiClient&) { return KeyOutcome::unhandled(); }

}  // namespace veloura
