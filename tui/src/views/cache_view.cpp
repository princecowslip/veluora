#include "cache_view.h"

#include "render_helpers.h"

namespace veloura {

void CacheView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/cache/status");
  if (response.ok()) status_ = response.body;
}

void CacheView::render(ncplane* plane, unsigned rows, unsigned cols) {
  (void)cols;
  int y = 0;
  print_plain(plane, y++, 0, "Cache");
  ++y;

  if (editing_quota_) {
    print_plain(plane, y++, 0, "New quota (MB): " + quota_input_ + "_");
    print_plain(plane, y++, 0, "Enter to set, Esc to cancel");
    return;
  }

  if (!status_.is_object()) {
    print_plain(plane, y++, 0, "(could not load cache status)");
    return;
  }

  const auto& breakdown = status_["breakdown"];
  print_plain(plane, y++, 0,
              "Thumbnails: " + std::to_string(static_cast<long>(bytes_to_mb(breakdown.value("thumbnails_bytes", 0ULL)))) +
                  " MB");
  print_plain(plane, y++, 0,
              "Stories:    " + std::to_string(static_cast<long>(bytes_to_mb(breakdown.value("stories_bytes", 0ULL)))) +
                  " MB");
  print_plain(plane, y++, 0,
              "Other:      " + std::to_string(static_cast<long>(bytes_to_mb(breakdown.value("other_bytes", 0ULL)))) +
                  " MB");
  print_plain(plane, y++, 0,
              "Total:      " + std::to_string(static_cast<long>(bytes_to_mb(breakdown.value("total_bytes", 0ULL)))) +
                  " MB");
  ++y;

  if (status_.contains("quota_bytes") && !status_["quota_bytes"].is_null()) {
    print_plain(plane, y++, 0,
                "Quota: " + std::to_string(static_cast<long>(bytes_to_mb(status_["quota_bytes"].get<std::uint64_t>()))) +
                    " MB");
  } else {
    print_plain(plane, y++, 0, "Quota: unlimited");
  }

  if (static_cast<unsigned>(y) < rows) {
    ++y;
    print_plain(plane, y++, 0, "s: set quota   c: clear quota   e: enforce quota now");
  }
}

KeyOutcome CacheView::handle_key(const ncinput& input, ApiClient& api) {
  if (editing_quota_) {
    if (input.id == NCKEY_ENTER) {
      editing_quota_ = false;
      try {
        const unsigned long long mb = std::stoull(quota_input_);
        auto response = api.post("/api/v1/cache/quota", {{"bytes", mb * 1024ULL * 1024ULL}});
        status_message = response.ok() ? "Quota set to " + quota_input_ + " MB." : "Could not set quota.";
      } catch (const std::exception&) {
        status_message = "Enter a whole number of MB.";
      }
      quota_input_.clear();
      refresh(api);
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_ESC) {
      editing_quota_ = false;
      quota_input_.clear();
      return KeyOutcome::handled();
    }
    if (input.id == NCKEY_BACKSPACE || input.id == 127) {
      if (!quota_input_.empty()) quota_input_.pop_back();
      return KeyOutcome::handled();
    }
    if (input.id >= '0' && input.id <= '9') {
      quota_input_.push_back(static_cast<char>(input.id));
      return KeyOutcome::handled();
    }
    return KeyOutcome::handled();
  }

  if (input.id == 's') {
    editing_quota_ = true;
    return KeyOutcome::handled();
  }
  if (input.id == 'c') {
    auto response = api.post("/api/v1/cache/quota", {{"bytes", nullptr}});
    status_message = response.ok() ? "Quota cleared (unlimited)." : "Could not clear quota.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'e') {
    auto response = api.post("/api/v1/cache/enforce-quota");
    if (response.ok()) {
      status_message = "Evicted " + std::to_string(response.body.value("evicted_files", 0)) + " file(s).";
    } else {
      status_message = "Could not enforce quota.";
    }
    refresh(api);
    return KeyOutcome::handled();
  }
  return KeyOutcome::unhandled();
}

}  // namespace veloura
