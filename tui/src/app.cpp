#include "app.h"

#include <atomic>
#include <csignal>

#include "views/render_helpers.h"

namespace veloura {

namespace {

// Set by SIGINT/SIGTERM handlers to request an orderly shutdown —
// per `docs/09-terminal-ui.md`: "Signal handling should request
// orderly shutdown and avoid doing complex work inside the signal
// handler." The main loop polls this between timed `notcurses_get()`
// calls rather than doing anything notcurses-related inside the
// handler itself.
std::atomic<bool> g_should_exit{false};

void handle_signal(int) { g_should_exit.store(true); }

bool is_ctrl(const ncinput& input, char letter) {
  const auto ctrl_code = static_cast<std::uint32_t>(letter - 'a' + 1);
  return input.id == ctrl_code || (input.id == static_cast<std::uint32_t>(letter) &&
                                    (input.modifiers & NCKEY_MOD_CTRL) != 0);
}

}  // namespace

App::App(std::string base_url, std::string token) : api_(std::move(base_url), std::move(token)) {
  notcurses_options options{};
  options.flags = NCOPTION_SUPPRESS_BANNERS | NCOPTION_NO_QUIT_SIGHANDLERS;
  options.loglevel = NCLOGLEVEL_WARNING;
  // `notcurses_init()` lives in the full multimedia `libnotcurses`;
  // since this build only links `notcurses-core` (no bitmap/FFmpeg
  // decode needed — Tier A is deferred), it must call the core-only
  // entry point instead.
  nc_ = notcurses_core_init(&options, stdout);

  tier_ = nc_ != nullptr ? detect_capability_tier(nc_) : CapabilityTier::TierC;

  home_view_ = std::make_unique<HomeView>();
  library_view_ = std::make_unique<LibraryView>();
  collections_view_ = std::make_unique<CollectionsView>();
  cache_view_ = std::make_unique<CacheView>();
  privacy_view_ = std::make_unique<PrivacyView>();
  diagnostics_view_ = std::make_unique<DiagnosticsView>();
  sources_view_ = std::make_unique<SourcesView>();
  discover_view_ = std::make_unique<DiscoverView>();
  item_detail_view_ = std::make_unique<ItemDetailView>();
  diagnostics_view_->set_capability_tier_label(tier_label(tier_));
}

App::~App() {
  if (nc_ != nullptr) {
    notcurses_stop(nc_);
  }
}

void App::layout() {
  ncplane* std_plane = notcurses_stdplane(nc_);
  unsigned rows = 0;
  unsigned cols = 0;
  ncplane_dim_yx(std_plane, &rows, &cols);

  const unsigned content_rows = rows > 2 ? rows - 2 : 0;

  if (header_ == nullptr) {
    ncplane_options header_opts{};
    header_opts.y = 0;
    header_opts.x = 0;
    header_opts.rows = 1;
    header_opts.cols = cols;
    header_opts.name = "header";
    header_ = ncplane_create(std_plane, &header_opts);

    ncplane_options content_opts{};
    content_opts.y = 1;
    content_opts.x = 0;
    content_opts.rows = content_rows > 0 ? content_rows : 1;
    content_opts.cols = cols;
    content_opts.name = "content";
    content_ = ncplane_create(std_plane, &content_opts);

    ncplane_options status_opts{};
    status_opts.y = static_cast<int>(rows > 0 ? rows - 1 : 0);
    status_opts.x = 0;
    status_opts.rows = 1;
    status_opts.cols = cols;
    status_opts.name = "status";
    status_ = ncplane_create(std_plane, &status_opts);
  } else {
    ncplane_resize_simple(header_, 1, cols);
    ncplane_resize_simple(content_, content_rows > 0 ? content_rows : 1, cols);
    ncplane_move_yx(content_, 1, 0);
    ncplane_resize_simple(status_, 1, cols);
    ncplane_move_yx(status_, static_cast<int>(rows > 0 ? rows - 1 : 0), 0);
  }
  dirty_ = true;
}

View* App::active_view() {
  switch (active_) {
    case ViewId::Home:
      return home_view_.get();
    case ViewId::Library:
      return library_view_.get();
    case ViewId::Collections:
      return collections_view_.get();
    case ViewId::Cache:
      return cache_view_.get();
    case ViewId::Privacy:
      return privacy_view_.get();
    case ViewId::Diagnostics:
      return diagnostics_view_.get();
    case ViewId::Sources:
      return sources_view_.get();
    case ViewId::Discover:
      return discover_view_.get();
    case ViewId::ItemDetail:
      return item_detail_view_.get();
  }
  return home_view_.get();
}

void App::switch_view(ViewId id) {
  if (id == active_) return;
  previous_ = active_;
  active_ = id;
  active_view()->refresh(api_);
  dirty_ = true;
}

void App::try_lock() {
  auto response = api_.get("/api/v1/privacy/status");
  has_password_ = response.ok() && response.body.value("has_password", false);
  if (has_password_) {
    locked_ = true;
    lock_password_input_.clear();
    lock_error_.clear();
  } else {
    home_view_->status_message = "No profile password is set — nothing to lock.";
  }
  dirty_ = true;
}

void App::handle_lock_key(const ncinput& input) {
  if (input.id == NCKEY_ENTER) {
    auto response = api_.post("/api/v1/privacy/verify", {{"password", lock_password_input_}});
    if (response.ok() && response.body.value("ok", false)) {
      locked_ = false;
      lock_password_input_.clear();
      lock_error_.clear();
    } else {
      lock_error_ = "Incorrect password.";
      lock_password_input_.clear();
    }
    dirty_ = true;
    return;
  }
  if (input.id == NCKEY_BACKSPACE || input.id == 127) {
    if (!lock_password_input_.empty()) lock_password_input_.pop_back();
    dirty_ = true;
    return;
  }
  if (input.id >= 0x20 && input.id < 0x7f) {
    lock_password_input_.push_back(static_cast<char>(input.id));
    dirty_ = true;
  }
}

void App::render_frame() {
  ncplane_erase(header_);
  ncplane_erase(content_);
  ncplane_erase(status_);

  unsigned header_rows = 0, header_cols = 0;
  ncplane_dim_yx(header_, &header_rows, &header_cols);
  std::string header_text = std::string("veloura-tui — ") + active_view()->title();
  print_plain(header_, 0, 0, header_text);

  unsigned content_rows = 0, content_cols = 0;
  ncplane_dim_yx(content_, &content_rows, &content_cols);

  if (content_cols < 60 || content_rows < 18) {
    print_plain(content_, 0, 0, "Terminal too small — resize to at least 60x18.");
  } else if (locked_) {
    print_plain(content_, 0, 0, "Session locked — enter password and press Enter.");
    std::string masked(lock_password_input_.size(), '*');
    print_plain(content_, 2, 0, "Password: " + masked);
    if (!lock_error_.empty()) print_plain(content_, 4, 0, lock_error_);
  } else if (help_visible_) {
    print_plain(content_, 0, 0, "Keybindings");
    print_plain(content_, 2, 0,
                "F1 Home   F2 Library   F3 Collections   F4 Downloads/Cache   F5 Privacy   F6 Diagnostics   F7 Sources   F8 Discover");
    print_plain(content_, 3, 0, "j/k or Up/Down   navigate     Enter/Space   open/select     Esc   back / cancel");
    print_plain(content_, 4, 0, "/   search (Library)          f   favorite (Item)         p   pin (Item)");
    print_plain(content_, 5, 0, "c   add to collection (Item)  Ctrl+L   lock                Q   quit");
    print_plain(content_, 6, 0, "? closes this help.");
  } else {
    active_view()->render(content_, content_rows, content_cols);
  }

  std::string status_line = locked_ ? "LOCKED" : std::string("[") + tier_label(tier_) + "]";
  if (!locked_ && !active_view()->status_message.empty()) {
    status_line += "  " + active_view()->status_message;
  }
  status_line += "   ?: help   Q: quit";
  print_plain(status_, 0, 0, status_line);

  notcurses_render(nc_);
}

void App::handle_input(const ncinput& input) {
  if (input.evtype == NCTYPE_RELEASE) return;

  if (input.id == NCKEY_RESIZE) {
    layout();
    return;
  }

  if (locked_) {
    handle_lock_key(input);
    return;
  }

  if (help_visible_) {
    if (input.id == NCKEY_ESC || input.id == '?') {
      help_visible_ = false;
      dirty_ = true;
    }
    return;
  }

  if (is_ctrl(input, 'l')) {
    try_lock();
    return;
  }

  KeyOutcome outcome = active_view()->handle_key(input, api_);
  if (outcome.consumed) {
    dirty_ = true;
    if (outcome.open_item_id.has_value()) {
      item_detail_view_->load(api_, *outcome.open_item_id);
      switch_view(ViewId::ItemDetail);
    }
    return;
  }

  if (input.id == 'Q') {
    running_ = false;
    return;
  }
  if (input.id == '?') {
    help_visible_ = true;
    dirty_ = true;
    return;
  }
  if (input.id == NCKEY_ESC && active_ == ViewId::ItemDetail) {
    switch_view(previous_);
    return;
  }
  if (input.id == NCKEY_F01) {
    switch_view(ViewId::Home);
    return;
  }
  if (input.id == NCKEY_F02) {
    switch_view(ViewId::Library);
    return;
  }
  if (input.id == NCKEY_F03) {
    switch_view(ViewId::Collections);
    return;
  }
  if (input.id == NCKEY_F04) {
    switch_view(ViewId::Cache);
    return;
  }
  if (input.id == NCKEY_F05) {
    switch_view(ViewId::Privacy);
    return;
  }
  if (input.id == NCKEY_F06) {
    switch_view(ViewId::Diagnostics);
    return;
  }
  if (input.id == NCKEY_F07) {
    switch_view(ViewId::Sources);
    return;
  }
  if (input.id == NCKEY_F08) {
    switch_view(ViewId::Discover);
    return;
  }
}

int App::run() {
  if (nc_ == nullptr) {
    fprintf(stderr, "could not initialize notcurses (is this a real terminal?)\n");
    return 1;
  }

  std::signal(SIGINT, handle_signal);
  std::signal(SIGTERM, handle_signal);

  layout();
  home_view_->refresh(api_);

  struct timespec timeout {
    0, 150'000'000
  };  // 150ms — frequent enough to notice a signal promptly, cheap enough idle.

  while (running_ && !g_should_exit.load()) {
    if (dirty_) {
      render_frame();
      dirty_ = false;
    }

    ncinput input{};
    const std::uint32_t id = notcurses_get(nc_, &timeout, &input);
    if (id == 0) {
      continue;  // timed out — loop back to check exit flags
    }
    if (id == static_cast<std::uint32_t>(-1)) {
      break;  // input error — exit rather than spin
    }
    handle_input(input);
  }

  return 0;
}

}  // namespace veloura
