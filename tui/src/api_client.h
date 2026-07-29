#pragma once

#include <cstdint>
#include <string>

#include <nlohmann/json.hpp>

namespace veloura {

// Response from a single request. `network_error` is set when the
// request never reached the server at all (connection refused, DNS,
// timeout) — distinct from an HTTP error status, which still carries a
// real `{"error": "..."}` JSON body from `local_api::error_response`.
struct ApiResponse {
  long status = 0;
  nlohmann::json body;
  bool network_error = false;
  std::string error_message;

  bool ok() const { return !network_error && status >= 200 && status < 300; }
};

// A thin, synchronous (blocking) libcurl + nlohmann::json client for
// `crates/local-api`'s loopback HTTP API. Blocking is a deliberate
// scope choice for this milestone: every call here is a direct
// consequence of a keypress against a loopback socket (sub-millisecond
// round trip), so a dedicated API worker thread — per
// `docs/09-terminal-ui.md`'s event-loop model — is deferred along with
// the event stream (`local-api` doesn't implement SSE/WebSockets yet
// either, so there's nothing to subscribe to regardless).
class ApiClient {
 public:
  ApiClient(std::string base_url, std::string token);

  ApiResponse get(const std::string& path);
  ApiResponse post(const std::string& path,
                    const nlohmann::json& body = nlohmann::json::object());
  ApiResponse del(const std::string& path);

 private:
  ApiResponse request(const std::string& method, const std::string& path,
                       const nlohmann::json* body);

  std::string base_url_;
  std::string token_;
};

}  // namespace veloura
