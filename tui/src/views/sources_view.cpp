#include "sources_view.h"

#include "render_helpers.h"

namespace veloura {

namespace {

// Matches `application::LOCAL_FILESYSTEM_CONNECTOR_ID` /
// `connectors::FEED_CONNECTOR_ID` / `connectors::BOORU_CONNECTOR_ID` /
// `connectors::OPDS_CONNECTOR_ID` — the four connectors that exist (see
// `crates/application/src/source.rs`, `crates/connectors/src/feed.rs`,
// `crates/connectors/src/booru.rs`, `crates/connectors/src/opds.rs`).
// Fixed values, not looked up from any "list connectors" endpoint,
// since none exists.
constexpr const char* kLocalFilesystemConnectorId = "00000000-0000-0000-0000-000000000000";
constexpr const char* kFeedConnectorId = "00000000-0000-0000-0000-000000000001";
constexpr const char* kBooruConnectorId = "00000000-0000-0000-0000-000000000002";
constexpr const char* kOpdsConnectorId = "00000000-0000-0000-0000-000000000003";

std::string connector_label(const std::string& connector_id) {
  if (connector_id == kLocalFilesystemConnectorId) return "Local filesystem";
  if (connector_id == kFeedConnectorId) return "RSS/Atom feed";
  if (connector_id == kBooruConnectorId) return "Booru (Danbooru/Gelbooru)";
  if (connector_id == kOpdsConnectorId) return "OPDS catalog";
  return "Unknown connector";
}

bool is_text_char(std::uint32_t id) { return id >= 0x20 && id < 0x7f; }

}  // namespace

void SourcesView::refresh(ApiClient& api) {
  auto response = api.get("/api/v1/sources");
  sources_.clear();
  if (response.ok() && response.body.is_array()) {
    for (const auto& s : response.body) sources_.push_back(s);
  }
  if (selected_row_ >= static_cast<int>(sources_.size())) selected_row_ = 0;
}

void SourcesView::reset_add_form() {
  adding_ = false;
  add_step_ = AddStep::ChooseConnector;
  add_connector_id_.clear();
  add_feed_url_input_.clear();
  add_booru_flavor_.clear();
  add_booru_base_url_input_.clear();
  add_booru_api_key_input_.clear();
  add_opds_url_input_.clear();
  add_opds_username_input_.clear();
  add_opds_password_input_.clear();
  add_display_name_input_.clear();
}

void SourcesView::render(ncplane* plane, unsigned rows, unsigned cols) {
  if (adding_) {
    render_add_form(plane, rows, cols);
  } else if (browsing_) {
    render_browse(plane, rows, cols);
  } else {
    render_list(plane, rows, cols);
  }
}

void SourcesView::render_list(ncplane* plane, unsigned rows, unsigned cols) {
  print_plain(plane, 0, 0, "Sources — a: add   e: enable/disable   h: health-check   b: browse   x: remove");
  if (sources_.empty()) {
    print_plain(plane, 2, 0, "(no sources configured yet — press a to add one)");
    return;
  }
  for (std::size_t i = 0; i < sources_.size() && 2 + i < rows; ++i) {
    const auto& s = sources_[i];
    std::string label = s.value("display_name", "?");
    label += "  [" + connector_label(s.value("connector_id", "")) + "]";
    label += s.value("enabled", false) ? "  enabled" : "  disabled";
    label += "  (" + s.value("health_state", std::string("unknown")) + ")";
    print_row(plane, static_cast<int>(2 + i), cols, label, selected_row_ == static_cast<int>(i));
  }

  if (delete_confirm_armed_ && !sources_.empty()) {
    const unsigned confirm_row = 3 + static_cast<unsigned>(sources_.size());
    if (confirm_row < rows) {
      print_plain(plane, static_cast<int>(confirm_row), 0,
                  "Remove '" + sources_[static_cast<std::size_t>(selected_row_)].value("display_name", "?") +
                      "'? Press x again to confirm, Esc to cancel.");
    }
  }
}

void SourcesView::render_add_form(ncplane* plane, unsigned rows, unsigned cols) {
  (void)cols;
  (void)rows;
  print_plain(plane, 0, 0, "Add source");
  switch (add_step_) {
    case AddStep::ChooseConnector:
      print_plain(plane, 2, 0,
                  "l: local filesystem   f: RSS/Atom feed   d: booru   o: OPDS catalog   Esc: cancel");
      break;
    case AddStep::FeedUrl:
      print_plain(plane, 2, 0, "Feed URL: " + add_feed_url_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::BooruFlavor:
      print_plain(plane, 2, 0, "d: Danbooru-compatible   g: Gelbooru-compatible   Esc: cancel");
      break;
    case AddStep::BooruBaseUrl:
      print_plain(plane, 2, 0, "Base URL: " + add_booru_base_url_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::BooruApiKey:
      print_plain(plane, 2, 0, "API key (optional): " + add_booru_api_key_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::OpdsUrl:
      print_plain(plane, 2, 0, "Catalog URL: " + add_opds_url_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::OpdsUsername:
      print_plain(plane, 2, 0, "Username (optional): " + add_opds_username_input_ + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    case AddStep::OpdsPassword: {
      std::string masked(add_opds_password_input_.size(), '*');
      print_plain(plane, 2, 0, "Password (optional): " + masked + "_");
      print_plain(plane, 4, 0, "Enter to continue, Esc to cancel");
      break;
    }
    case AddStep::DisplayName:
      print_plain(plane, 2, 0, "Display name: " + add_display_name_input_ + "_");
      print_plain(plane, 4, 0, "Enter to add, Esc to cancel");
      break;
  }
}

void SourcesView::render_browse(ncplane* plane, unsigned rows, unsigned cols) {
  print_plain(plane, 0, 0, "Browsing " + browse_source_name_ + " — i: import   Esc: back");
  if (browse_results_.empty()) {
    print_plain(plane, 2, 0, "(no items)");
    return;
  }
  for (std::size_t i = 0; i < browse_results_.size() && 2 + i < rows; ++i) {
    std::string label = browse_results_[i].value("title", "(untitled)");
    print_row(plane, static_cast<int>(2 + i), cols, label, browse_selected_row_ == static_cast<int>(i));
  }
}

KeyOutcome SourcesView::handle_key(const ncinput& input, ApiClient& api) {
  if (adding_) return handle_add_form_key(input, api);
  if (browsing_) return handle_browse_key(input, api);
  return handle_list_key(input, api);
}

KeyOutcome SourcesView::handle_list_key(const ncinput& input, ApiClient& api) {
  if (input.id == 'a') {
    reset_add_form();
    adding_ = true;
    return KeyOutcome::handled();
  }

  if (sources_.empty()) return KeyOutcome::unhandled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    selected_row_ = (selected_row_ + 1) % static_cast<int>(sources_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    selected_row_ =
        (selected_row_ - 1 + static_cast<int>(sources_.size())) % static_cast<int>(sources_.size());
    delete_confirm_armed_ = false;
    return KeyOutcome::handled();
  }

  const auto& selected = sources_[static_cast<std::size_t>(selected_row_)];
  const std::string id = selected.value("id", "");

  if (input.id == 'e') {
    const bool enabled = selected.value("enabled", false);
    auto response = api.post(std::string("/api/v1/sources/") + id + (enabled ? "/disable" : "/enable"));
    status_message = response.ok() ? (enabled ? "Source disabled." : "Source enabled.")
                                    : "Could not update source.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'h') {
    auto response = api.post("/api/v1/sources/" + id + "/health-check");
    status_message =
        response.ok() && response.body.is_string() ? "Health check: " + response.body.get<std::string>() + "."
                                                     : "Health check failed.";
    refresh(api);
    return KeyOutcome::handled();
  }
  if (input.id == 'b') {
    auto response = api.get("/api/v1/sources/" + id + "/browse");
    browse_results_.clear();
    browse_source_id_ = id;
    browse_source_name_ = selected.value("display_name", "?");
    browse_selected_row_ = 0;
    if (response.ok() && response.body.is_object() && response.body.contains("result")) {
      const auto& result = response.body["result"];
      const std::string status = result.value("status", "");
      if ((status == "success" || status == "partial") && result.contains("data") &&
          result["data"].is_array()) {
        for (const auto& item : result["data"]) browse_results_.push_back(item);
        status_message.clear();
      } else {
        status_message = "Browse result: " + status;
      }
    } else {
      status_message = "Could not browse source.";
    }
    browsing_ = true;
    return KeyOutcome::handled();
  }
  if (input.id == 'x') {
    if (delete_confirm_armed_) {
      api.del("/api/v1/sources/" + id);
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

KeyOutcome SourcesView::handle_add_form_key(const ncinput& input, ApiClient& api) {
  if (input.id == NCKEY_ESC) {
    reset_add_form();
    return KeyOutcome::handled();
  }

  switch (add_step_) {
    case AddStep::ChooseConnector:
      if (input.id == 'l') {
        add_connector_id_ = kLocalFilesystemConnectorId;
        add_step_ = AddStep::DisplayName;
        return KeyOutcome::handled();
      }
      if (input.id == 'f') {
        add_connector_id_ = kFeedConnectorId;
        add_step_ = AddStep::FeedUrl;
        return KeyOutcome::handled();
      }
      if (input.id == 'd') {
        add_connector_id_ = kBooruConnectorId;
        add_step_ = AddStep::BooruFlavor;
        return KeyOutcome::handled();
      }
      if (input.id == 'o') {
        add_connector_id_ = kOpdsConnectorId;
        add_step_ = AddStep::OpdsUrl;
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::FeedUrl:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::DisplayName;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_feed_url_input_.empty()) add_feed_url_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_feed_url_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::BooruFlavor:
      if (input.id == 'd') {
        add_booru_flavor_ = "danbooru";
        add_step_ = AddStep::BooruBaseUrl;
        return KeyOutcome::handled();
      }
      if (input.id == 'g') {
        add_booru_flavor_ = "gelbooru";
        add_step_ = AddStep::BooruBaseUrl;
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::BooruBaseUrl:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::BooruApiKey;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_booru_base_url_input_.empty()) add_booru_base_url_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_booru_base_url_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::BooruApiKey:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::DisplayName;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_booru_api_key_input_.empty()) add_booru_api_key_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_booru_api_key_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::OpdsUrl:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::OpdsUsername;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_opds_url_input_.empty()) add_opds_url_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_opds_url_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::OpdsUsername:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::OpdsPassword;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_opds_username_input_.empty()) add_opds_username_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_opds_username_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::OpdsPassword:
      if (input.id == NCKEY_ENTER) {
        add_step_ = AddStep::DisplayName;
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_opds_password_input_.empty()) add_opds_password_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_opds_password_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();

    case AddStep::DisplayName:
      if (input.id == NCKEY_ENTER) {
        nlohmann::json configuration_json = nlohmann::json::object();
        if (add_connector_id_ == kFeedConnectorId) {
          configuration_json = {{"url", add_feed_url_input_}};
        } else if (add_connector_id_ == kBooruConnectorId) {
          configuration_json = {{"flavor", add_booru_flavor_}, {"base_url", add_booru_base_url_input_}};
          if (!add_booru_api_key_input_.empty()) {
            configuration_json["api_key"] = add_booru_api_key_input_;
          }
        } else if (add_connector_id_ == kOpdsConnectorId) {
          configuration_json = {{"url", add_opds_url_input_}};
          if (!add_opds_username_input_.empty()) {
            configuration_json["username"] = add_opds_username_input_;
          }
          if (!add_opds_password_input_.empty()) {
            configuration_json["password"] = add_opds_password_input_;
          }
        }
        auto response = api.post("/api/v1/sources", {{"connector_id", add_connector_id_},
                                                       {"display_name", add_display_name_input_},
                                                       {"configuration_json", configuration_json}});
        status_message = response.ok() ? "Source added." : "Could not add source.";
        reset_add_form();
        refresh(api);
        return KeyOutcome::handled();
      }
      if (input.id == NCKEY_BACKSPACE || input.id == 127) {
        if (!add_display_name_input_.empty()) add_display_name_input_.pop_back();
        return KeyOutcome::handled();
      }
      if (is_text_char(input.id)) {
        add_display_name_input_.push_back(static_cast<char>(input.id));
        return KeyOutcome::handled();
      }
      return KeyOutcome::handled();
  }
  return KeyOutcome::handled();
}

KeyOutcome SourcesView::handle_browse_key(const ncinput& input, ApiClient& api) {
  if (input.id == NCKEY_ESC) {
    browsing_ = false;
    return KeyOutcome::handled();
  }

  if (browse_results_.empty()) return KeyOutcome::handled();

  if (input.id == NCKEY_DOWN || input.id == 'j') {
    browse_selected_row_ = (browse_selected_row_ + 1) % static_cast<int>(browse_results_.size());
    return KeyOutcome::handled();
  }
  if (input.id == NCKEY_UP || input.id == 'k') {
    browse_selected_row_ = (browse_selected_row_ - 1 + static_cast<int>(browse_results_.size())) %
                            static_cast<int>(browse_results_.size());
    return KeyOutcome::handled();
  }
  if (input.id == 'i') {
    const auto& item = browse_results_[static_cast<std::size_t>(browse_selected_row_)];
    auto response = api.post("/api/v1/sources/" + browse_source_id_ + "/import", item);
    status_message = response.ok() ? "Imported into the library." : "Could not import item.";
    return KeyOutcome::handled();
  }
  return KeyOutcome::handled();
}

}  // namespace veloura
