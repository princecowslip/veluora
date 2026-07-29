#include "api_client.h"

#include <curl/curl.h>

namespace veloura {

namespace {

std::size_t write_callback(char* ptr, std::size_t size, std::size_t nmemb, void* userdata) {
  auto* out = static_cast<std::string*>(userdata);
  out->append(ptr, size * nmemb);
  return size * nmemb;
}

}  // namespace

ApiClient::ApiClient(std::string base_url, std::string token)
    : base_url_(std::move(base_url)), token_(std::move(token)) {}

ApiResponse ApiClient::get(const std::string& path) { return request("GET", path, nullptr); }

ApiResponse ApiClient::post(const std::string& path, const nlohmann::json& body) {
  return request("POST", path, &body);
}

ApiResponse ApiClient::del(const std::string& path) { return request("DELETE", path, nullptr); }

ApiResponse ApiClient::request(const std::string& method, const std::string& path,
                                const nlohmann::json* body) {
  ApiResponse response;

  CURL* curl = curl_easy_init();
  if (curl == nullptr) {
    response.network_error = true;
    response.error_message = "could not initialize curl";
    return response;
  }

  const std::string url = base_url_ + path;
  std::string response_body;
  std::string request_body;

  struct curl_slist* headers = nullptr;
  const std::string auth_header = "Authorization: Bearer " + token_;
  headers = curl_slist_append(headers, auth_header.c_str());

  curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response_body);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT_MS, 5000L);
  curl_easy_setopt(curl, CURLOPT_NOSIGNAL, 1L);

  if (method == "GET") {
    // default
  } else if (method == "DELETE") {
    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "DELETE");
  } else if (method == "POST") {
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    request_body = body != nullptr ? body->dump() : "{}";
    headers = curl_slist_append(headers, "Content-Type: application/json");
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, request_body.c_str());
    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, static_cast<long>(request_body.size()));
  }

  curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);

  CURLcode result = curl_easy_perform(curl);
  if (result != CURLE_OK) {
    response.network_error = true;
    response.error_message = curl_easy_strerror(result);
  } else {
    long status = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    response.status = status;
    if (!response_body.empty()) {
      try {
        response.body = nlohmann::json::parse(response_body);
      } catch (const nlohmann::json::parse_error&) {
        // Non-JSON body (shouldn't happen against this API) — leave
        // `body` null rather than crash the TUI over a malformed
        // response.
      }
    }
  }

  curl_slist_free_all(headers);
  curl_easy_cleanup(curl);
  return response;
}

}  // namespace veloura
