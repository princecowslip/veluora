#pragma once

#include <notcurses/notcurses.h>

#include <array>
#include <memory>
#include <string>

#include "api_client.h"
#include "capability.h"
#include "views/cache_view.h"
#include "views/collections_view.h"
#include "views/diagnostics_view.h"
#include "views/home_view.h"
#include "views/item_detail_view.h"
#include "views/library_view.h"
#include "views/privacy_view.h"
#include "views/view.h"

namespace veloura {

enum class ViewId {
  Home,
  Library,
  Collections,
  Cache,
  Privacy,
  Diagnostics,
  ItemDetail,
};

// Owns the notcurses context, the plane hierarchy (a deliberately
// trimmed version of `docs/09-terminal-ui.md`'s full plane tree — see
// the tui/ plan for exactly what's cut and why), and the top-level
// input/render loop. Everything view-specific lives in `views/`;
// everything cross-cutting (navigation, the lock shield, help overlay,
// terminal lifecycle) lives here.
class App {
 public:
  App(std::string base_url, std::string token);
  ~App();

  App(const App&) = delete;
  App& operator=(const App&) = delete;

  // Returns an exit code suitable for `main()`.
  int run();

 private:
  void layout();
  void render_frame();
  void handle_input(const ncinput& input);
  void switch_view(ViewId id);
  View* active_view();
  void try_lock();
  void handle_lock_key(const ncinput& input);

  notcurses* nc_ = nullptr;
  ncplane* header_ = nullptr;
  ncplane* content_ = nullptr;
  ncplane* status_ = nullptr;

  ApiClient api_;
  CapabilityTier tier_;

  std::unique_ptr<HomeView> home_view_;
  std::unique_ptr<LibraryView> library_view_;
  std::unique_ptr<CollectionsView> collections_view_;
  std::unique_ptr<CacheView> cache_view_;
  std::unique_ptr<PrivacyView> privacy_view_;
  std::unique_ptr<DiagnosticsView> diagnostics_view_;
  std::unique_ptr<ItemDetailView> item_detail_view_;

  ViewId active_ = ViewId::Home;
  ViewId previous_ = ViewId::Home;
  bool running_ = true;
  bool dirty_ = true;
  bool help_visible_ = false;

  bool locked_ = false;
  bool has_password_ = false;
  std::string lock_password_input_;
  std::string lock_error_;
};

}  // namespace veloura
