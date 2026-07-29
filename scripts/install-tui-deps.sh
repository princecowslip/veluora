#!/usr/bin/env bash
# Installs the required build dependencies for the notcurses TUI
# (`tui/`), per `docs/45-required-packages-dependencies.md`'s
# Debian/Ubuntu "Preferred packaged installation" list. Required-only:
# the recommended multimedia packages (FFmpeg, etc.) aren't needed —
# Tier A bitmap thumbnail decoding is explicitly deferred past this
# milestone, so the TUI never decodes media itself.
set -euo pipefail

sudo apt update
sudo apt install -y \
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
