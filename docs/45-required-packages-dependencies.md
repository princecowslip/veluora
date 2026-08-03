# Required Packages and Dependencies

## Dependency policy

Dependencies are divided into:

- Required build tools
- Required TUI libraries
- Required shared application libraries
- Recommended media libraries
- Optional capabilities
- Development and test tools

Exact versions should be pinned in release manifests after CI validation. Distribution package names vary.

## TUI architecture

The TUI is a C++20 client using notcurses 3.x.

Required direct TUI dependencies:

| Dependency | Purpose | Required |
|---|---|---:|
| notcurses / notcurses-core | Terminal rendering, input, planes and visuals | Yes |
| terminfo / ncurses development files | Terminal capabilities used by notcurses | Yes |
| libunistring | Unicode support used by notcurses | Yes |
| C++20 standard library | TUI application code | Yes |
| CMake 3.21+ | Configure the TUI build | Yes |
| pkg-config or pkgconf | Resolve notcurses and native libraries | Yes |
| Ninja or Make | Build execution | Yes |
| libcurl | Local HTTP/API and event transport when HTTP is used | Yes for HTTP transport |
| nlohmann-json | JSON request and response parsing | Yes for JSON transport |
| Threads | Background API and thumbnail workers | Yes |

notcurses itself currently documents CMake 3.21+, a C17 compiler, terminfo 6.1+, and libunistring 0.9.10+ as core requirements. The Veloura client uses the C API from C++20.

## Recommended notcurses multimedia dependencies

For inline TUI images, animated previews, and broad media decoding:

| Dependency | Purpose |
|---|---|
| FFmpeg libavformat | Container and media input |
| FFmpeg libavutil | Shared media utilities |
| FFmpeg libavdevice | Input device support used by multimedia builds |
| FFmpeg libswscale | Pixel format conversion and scaling |
| libdeflate | Efficient compressed data handling where packaged |
| OpenImageIO | Optional still-image backend; not required for MVP |
| libgpm | Optional Linux console mouse support |
| libqrcodegen | Optional QR widget support |

Veloura does not require OpenImageIO or QR support for the MVP.

## Shared application dependencies

This is the recommended baseline for the complete project, as a
cross-language design target. The shipped Rust core (`crates/`) resolves
most of this table through Cargo crates rather than system packages, so
none of the following need installing to build or run it today:

| Dependency in this table | What the Rust core actually uses instead |
|---|---|
| SQLite 3 | `rusqlite` with the `bundled` feature — compiles SQLite from source, no `libsqlite3-dev` needed |
| OpenSSL or platform TLS | `reqwest` with the `rustls` feature — no system OpenSSL needed |
| libarchive | The pure-Rust `zip` crate (`crates/media/src/archive.rs`) |
| libmagic | The pure-Rust `mime_guess` crate |
| ICU or equivalent | Not used — no ICU dependency exists anywhere in the workspace |
| OS credential store adapter | Not implemented yet — connector credentials are stored in plain `configuration_json` (see `KNOWN_ISSUES.md`) |

FFmpeg is invoked as an external CLI binary (`ffprobe`/`ffmpeg` on `PATH`,
checked via `DiagnosticsService`), not linked via `libavformat`/`libavutil`
dev headers — CI installs the plain `ffmpeg` package, not `-dev` headers.

The table below remains accurate for the C++ TUI's own direct
dependencies (see "TUI architecture" above) and as the design target for a
future unified packaging story:

| Dependency | Purpose | Requirement |
|---|---|---|
| SQLite 3 | Local metadata and user state | Required |
| SQLite FTS5 | Full-text search | Required |
| OpenSSL or platform TLS | HTTPS and cryptographic primitives | Required |
| libcurl or equivalent HTTP client | Connector networking | Required |
| libarchive | Safe archive reading for comics and imports | Required |
| zlib and zstd | Compressed metadata and archives | Required |
| libmagic | MIME and file-type inspection | Recommended |
| FFmpeg | Media probing and playback integration | Required for full media support |
| mpv or libmpv | Optional external or embedded video playback | Recommended |
| ICU or equivalent | Advanced Unicode, collation and BiDi | Recommended |
| OS credential store adapter | Secret storage | Required |
| JSON library | Configuration and connector messages | Required |
| TOML library | Human-editable configuration | Recommended |

## Recommended implementation stack

The dependency plan assumes:

```text
Core service: Rust or another memory-safe compiled language
Desktop GUI: chosen cross-platform GUI shell
TUI: C++20 + notcurses
CLI: shared core client
IPC: local socket or authenticated loopback HTTP
Database: SQLite
Media: FFmpeg + external/embedded mpv
```

The TUI remains language-isolated from the core through IPC, avoiding notcurses FFI in the core.

## Debian and Ubuntu

### Preferred packaged installation

Package availability varies by release. On releases that provide notcurses development packages:

```bash
sudo apt update
sudo apt install \
  build-essential \
  cmake \
  ninja-build \
  pkg-config \
  libnotcurses-dev \
  libnotcurses-core-dev \
  libunistring-dev \
  libncurses-dev \
  libcurl4-openssl-dev \
  nlohmann-json3-dev
```

Recommended multimedia and application packages:

```bash
sudo apt install \
  ffmpeg \
  libavformat-dev \
  libavutil-dev \
  libavdevice-dev \
  libswscale-dev \
  libdeflate-dev \
  libarchive-dev \
  libsqlite3-dev \
  libssl-dev \
  libmagic-dev \
  libzstd-dev \
  zlib1g-dev \
  libicu-dev \
  libmpv-dev
```

Optional:

```bash
sudo apt install \
  libgpm-dev \
  libqrcodegen-dev \
  libopenimageio-dev \
  doctest-dev \
  clang-tidy \
  cppcheck \
  valgrind
```

If the distribution does not provide a sufficiently recent notcurses package, build notcurses from source using the upstream requirements.

## Fedora and RHEL-family systems

```bash
sudo dnf install \
  gcc \
  gcc-c++ \
  cmake \
  ninja-build \
  pkgconf-pkg-config \
  notcurses \
  notcurses-devel \
  libunistring-devel \
  ncurses-devel \
  libcurl-devel \
  json-devel
```

Recommended:

```bash
sudo dnf install \
  ffmpeg \
  ffmpeg-devel \
  libdeflate-devel \
  libarchive-devel \
  sqlite-devel \
  openssl-devel \
  file-devel \
  libzstd-devel \
  zlib-devel \
  libicu-devel \
  mpv-devel
```

FFmpeg and mpv package availability may require a distribution-approved third-party multimedia repository. Packaging policy must be documented rather than silently adding repositories.

## Arch Linux

```bash
sudo pacman -S --needed \
  base-devel \
  cmake \
  ninja \
  pkgconf \
  notcurses \
  libunistring \
  ncurses \
  curl \
  nlohmann-json \
  ffmpeg \
  libdeflate \
  libarchive \
  sqlite \
  openssl \
  file \
  zstd \
  zlib \
  icu \
  mpv
```

Development headers are generally included with Arch packages.

## macOS with Homebrew

```bash
brew install \
  cmake \
  ninja \
  pkgconf \
  notcurses \
  curl \
  nlohmann-json \
  sqlite \
  openssl@3 \
  libarchive \
  zstd \
  libmagic \
  icu4c \
  ffmpeg \
  mpv
```

Homebrew's notcurses formula installs notcurses and its packaged dependencies. CMake may need explicit prefixes for keg-only libraries such as OpenSSL or ICU.

Example:

```bash
cmake -S tui -B build/tui -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo \
  -DOPENSSL_ROOT_DIR="$(brew --prefix openssl@3)" \
  -DCMAKE_PREFIX_PATH="$(brew --prefix icu4c)"
```

## Windows

### Tier 1 recommendation: WSL

Use Ubuntu or another supported Linux distribution under WSL and follow the Linux package instructions.

Recommended terminal:

- Windows Terminal

### Native Windows

notcurses has current Windows CI, but Veloura should treat native packaging as Tier 2 until tested.

Potential build environment:

- MSYS2 UCRT64
- CMake
- Ninja
- GCC or Clang
- notcurses and dependencies built for the same runtime
- curl and JSON libraries from the same environment

Do not mix MSVC-built and MinGW-built libraries without a deliberate ABI boundary.

Native Windows release tasks:

- Package notcurses DLLs
- Package dependent DLLs
- Validate Windows Terminal input
- Validate Unicode and grapheme rendering
- Validate terminal restoration
- Test resize and focus events
- Sign binaries and installer

## FreeBSD

Indicative package set:

```bash
sudo pkg install \
  cmake \
  ninja \
  pkgconf \
  notcurses \
  curl \
  nlohmann-json \
  sqlite3 \
  openssl \
  libarchive \
  libmagic \
  zstd \
  ffmpeg \
  mpv
```

Package names and options must be confirmed in CI for each supported FreeBSD release.

## Building notcurses from source

Use only when a suitable package is unavailable.

Typical procedure:

```bash
git clone https://github.com/dankamongmen/notcurses.git
cd notcurses
cmake -S . -B build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DUSE_PANDOC=OFF \
  -DUSE_DOCTEST=OFF
cmake --build build
sudo cmake --install build
sudo ldconfig
```

Minimal build without multimedia:

```bash
cmake -S . -B build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DUSE_MULTIMEDIA=none \
  -DUSE_PANDOC=OFF \
  -DUSE_DOCTEST=OFF
```

Verify installation:

```bash
pkg-config --modversion notcurses
pkg-config --cflags --libs notcurses
notcurses-info
```

CMake option names must be validated against the selected notcurses release before pinning the build script.

## Veloura TUI build

```bash
cmake -S tui -B build/tui -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo

cmake --build build/tui
ctest --test-dir build/tui --output-on-failure
```

Run:

```bash
./build/tui/veloura-tui
```

## CMake linking

Preferred discovery:

```cmake
find_package(PkgConfig REQUIRED)
pkg_check_modules(NOTCURSES REQUIRED IMPORTED_TARGET notcurses)
```

Link:

```cmake
target_link_libraries(veloura-tui
  PRIVATE
    PkgConfig::NOTCURSES
    CURL::libcurl
    nlohmann_json::nlohmann_json
    Threads::Threads
)
```

Use `notcurses-core` only if Veloura deliberately disables multimedia visuals.

## Runtime dependencies

Required for the packaged TUI:

- notcurses shared library
- notcurses-core shared library where separately packaged
- terminfo database
- libunistring
- C++ runtime
- curl and TLS libraries when using HTTP IPC
- local Veloura service

Recommended:

- UTF-8 locale
- modern terminal with true colour
- Kitty, Sixel, or compatible graphics for inline previews
- mpv for external playback

## Development dependencies

Required:

- Git
- CMake
- Ninja
- C/C++ compiler
- pkg-config
- formatter
- test framework

Recommended:

- clang-format
- clang-tidy
- cppcheck
- AddressSanitizer
- UndefinedBehaviorSanitizer
- ThreadSanitizer in dedicated CI
- Valgrind on Linux
- gcov or llvm-cov
- CTest
- pre-commit
- shellcheck
- markdownlint

## CI matrix

```text
Ubuntu latest:
  GCC
  Clang
  multimedia on
  multimedia off
  ASan + UBSan

Fedora latest:
  packaged notcurses
  release build

macOS:
  Apple Silicon
  Homebrew dependencies

Windows:
  WSL
  native MSYS2 experimental

FreeBSD:
  smoke build
```

## Dependency update policy

- Track direct dependencies in `assets/dependencies.json` — exists today as
  a minimal stub, populated further as dependencies land.
- Pin release builds.
- Review notcurses release notes before updating.
- Re-run terminal capability tests after every notcurses update.
- Generate a software bill of materials.
- Scan native and language dependencies.
- Document optional features disabled by missing packages.
- Never download build dependencies during a release build without hashes.
