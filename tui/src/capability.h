#pragma once

#include <notcurses/notcurses.h>

#include <string>

namespace veloura {

// Only Tier B (Unicode, 24-bit/256-color) and Tier C (text-only) are
// distinguished — Tier A (Kitty/Sixel inline bitmap thumbnails) is
// explicitly deferred this milestone (see `tui/` plan), so it's never
// selected even on a terminal that could support it.
enum class CapabilityTier {
  TierB,
  TierC,
};

CapabilityTier detect_capability_tier(notcurses* nc);

std::string tier_label(CapabilityTier tier);

}  // namespace veloura
