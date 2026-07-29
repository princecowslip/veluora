#pragma once

#include <cstdint>
#include <optional>
#include <string>

namespace veloura {

// Mirrors `AppContext::open_default()`'s data-dir resolution
// (`directories::ProjectDirs::from("", "", "veloura")`), independently
// — the TUI is a separate process per `docs/12-system-architecture.md`'s
// notcurses TUI boundary and cannot link `crates/application`.
//
// The Linux path (`$XDG_DATA_HOME/veloura`, falling back to
// `~/.local/share/veloura`) was empirically confirmed against a real
// `veloura doctor` run earlier in this project. The macOS branch
// (`~/Library/Application Support/veloura`) is the best-effort mirror
// of the same empty-qualifier/empty-organization `directories` crate
// behavior, but is NOT verified against a real run — there is no macOS
// build environment available here. Verify it before relying on it in
// a release.
std::string resolve_data_dir();

struct Credentials {
  std::string token;
  std::uint16_t port;
};

// Reads `<data_dir>/api-token` and `<data_dir>/api-port`, written by
// `local-api`'s `write_token_file`/`write_port_file`. Returns
// `std::nullopt` if either file is missing or unreadable — the caller
// is expected to print a clear "is local-api running?" message rather
// than a raw file error.
std::optional<Credentials> load_credentials(const std::string& data_dir);

}  // namespace veloura
