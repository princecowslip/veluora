#pragma once

#include <nlohmann/json.hpp>

#include "view.h"

namespace veloura {

// The local-only reinterpretation of Milestone G's "downloads/offline"
// half: cache breakdown, quota get/set, and enforce-now — via
// `GET/POST /cache/*`. Thumbnail/generated-artifact cache only; the
// real remote download queue is `DownloadsView` (F9), added in
// Milestone J once connectors could actually download something.
class CacheView : public View {
 public:
  const char* title() const override { return "Cache"; }
  void refresh(ApiClient& api) override;
  void render(ncplane* plane, unsigned rows, unsigned cols) override;
  KeyOutcome handle_key(const ncinput& input, ApiClient& api) override;

 private:
  nlohmann::json status_;
  bool editing_quota_ = false;
  std::string quota_input_;
};

}  // namespace veloura
