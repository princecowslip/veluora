#pragma once

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// Read-only lock/encryption status (`GET /privacy/status`). Locking
// itself is a global app-level action (Ctrl+L, drawn as a full-plane
// shield above all content) rather than something this view triggers —
// see `App::try_lock`. There's no route to set a password from here;
// that stays a GUI/CLI action.
class PrivacyView : public View {
 public:
  const char* title() const override { return "Privacy"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  nlohmann::json status_;
};

}  // namespace veloura
