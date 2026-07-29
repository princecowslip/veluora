cmake_minimum_required(VERSION 3.21)
project(veloura-tui LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE RelWithDebInfo)
endif()

find_package(PkgConfig REQUIRED)
# notcurses-core (not the full notcurses/multimedia package) is enough:
# Tier A bitmap/image loading is explicitly deferred this milestone, so
# nothing here calls ncvisual_from_file() or needs FFmpeg-backed decode.
pkg_check_modules(NOTCURSES REQUIRED IMPORTED_TARGET notcurses-core)
pkg_check_modules(CURL REQUIRED IMPORTED_TARGET libcurl)
find_package(nlohmann_json 3.2 REQUIRED)
find_package(Threads REQUIRED)

add_executable(veloura-tui
  src/main.cpp
  src/config.cpp
  src/api_client.cpp
  src/capability.cpp
  src/app.cpp
  src/views/home_view.cpp
  src/views/library_view.cpp
  src/views/item_detail_view.cpp
  src/views/collections_view.cpp
  src/views/cache_view.cpp
  src/views/privacy_view.cpp
  src/views/diagnostics_view.cpp
  src/views/sources_view.cpp
)

target_compile_options(veloura-tui PRIVATE -Wall -Wextra)

target_link_libraries(veloura-tui PRIVATE
  PkgConfig::NOTCURSES
  PkgConfig::CURL
  nlohmann_json::nlohmann_json
  Threads::Threads
)
