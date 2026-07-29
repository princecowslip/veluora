#include "config.h"

#include <cstdlib>
#include <fstream>
#include <sstream>

namespace veloura {

std::string resolve_data_dir() {
  const char* home = std::getenv("HOME");
  const std::string home_str = home != nullptr ? home : "";

#if defined(__APPLE__)
  return home_str + "/Library/Application Support/veloura";
#else
  const char* xdg_data_home = std::getenv("XDG_DATA_HOME");
  if (xdg_data_home != nullptr && xdg_data_home[0] != '\0') {
    return std::string(xdg_data_home) + "/veloura";
  }
  return home_str + "/.local/share/veloura";
#endif
}

std::optional<Credentials> load_credentials(const std::string& data_dir) {
  std::ifstream token_file(data_dir + "/api-token");
  if (!token_file) {
    return std::nullopt;
  }
  std::ostringstream token_stream;
  token_stream << token_file.rdbuf();
  std::string token = token_stream.str();
  while (!token.empty() && (token.back() == '\n' || token.back() == '\r')) {
    token.pop_back();
  }
  if (token.empty()) {
    return std::nullopt;
  }

  std::ifstream port_file(data_dir + "/api-port");
  if (!port_file) {
    return std::nullopt;
  }
  int port = 0;
  port_file >> port;
  if (port <= 0 || port > 65535) {
    return std::nullopt;
  }

  return Credentials{std::move(token), static_cast<std::uint16_t>(port)};
}

}  // namespace veloura
