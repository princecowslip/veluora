#include <csignal>
#include <cstdio>

#include <curl/curl.h>

#include "app.h"
#include "config.h"

int main() {
  // Reap external-player children automatically (see
  // `ItemDetailView::open_target`'s `spawn_detached`) instead of
  // tracking pids and calling `waitpid` ourselves.
  std::signal(SIGCHLD, SIG_IGN);

  curl_global_init(CURL_GLOBAL_DEFAULT);

  const std::string data_dir = veloura::resolve_data_dir();
  const auto credentials = veloura::load_credentials(data_dir);
  if (!credentials.has_value()) {
    std::fprintf(stderr,
                 "Could not find veloura's local-api token/port files under %s.\n"
                 "Is `veloura-local-api` running?\n",
                 data_dir.c_str());
    curl_global_cleanup();
    return 1;
  }

  // Views' paths already carry the `/api/v1` prefix (matching
  // `crates/local-api`'s route table), so the base URL stops at the
  // port.
  const std::string base_url = "http://127.0.0.1:" + std::to_string(credentials->port);
  veloura::App app(base_url, credentials->token);
  const int result = app.run();

  curl_global_cleanup();
  return result;
}
