#pragma once

#include <notcurses/notcurses.h>

#include <string>

namespace veloura {

// Prints `text` at row `y`, padded/truncated to `cols`, highlighted
// (indigo-ish background per `docs/09-terminal-ui.md`'s color tokens)
// when `selected` — the shared list-row style every view's list uses.
inline void print_row(ncplane* plane, int y, unsigned cols, const std::string& text,
                       bool selected) {
  if (selected) {
    ncplane_set_bg_rgb8(plane, 60, 55, 100);
    ncplane_set_fg_rgb8(plane, 235, 235, 255);
  } else {
    ncplane_set_fg_default(plane);
    ncplane_set_bg_default(plane);
  }
  std::string line = text;
  if (line.size() > cols) {
    line = line.substr(0, cols);
  } else if (line.size() < cols) {
    line.append(cols - line.size(), ' ');
  }
  ncplane_putstr_yx(plane, y, 0, line.c_str());
  ncplane_set_fg_default(plane);
  ncplane_set_bg_default(plane);
}

inline void print_plain(ncplane* plane, int y, int x, const std::string& text) {
  ncplane_putstr_yx(plane, y, x, text.c_str());
}

inline double bytes_to_mb(std::uint64_t bytes) { return static_cast<double>(bytes) / (1024.0 * 1024.0); }

}  // namespace veloura
