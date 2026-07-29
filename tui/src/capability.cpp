#include "capability.h"

namespace veloura {

CapabilityTier detect_capability_tier(notcurses* nc) {
  return notcurses_canutf8(nc) ? CapabilityTier::TierB : CapabilityTier::TierC;
}

std::string tier_label(CapabilityTier tier) {
  switch (tier) {
    case CapabilityTier::TierB:
      return "Tier B (Unicode)";
    case CapabilityTier::TierC:
      return "Tier C (text-only)";
  }
  return "unknown";
}

}  // namespace veloura
