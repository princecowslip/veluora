#pragma once

#include <notcurses/notcurses.h>

#include <optional>
#include <string>

#include "../api_client.h"

namespace veloura {

// What handling a key produced, beyond "consumed or not". `open_item_id`
// lets Home/Library/Collections request the app switch to Item Detail
// for a specific item without each view needing to know about the
// app's view-switching mechanics itself.
struct KeyOutcome {
  bool consumed = false;
  std::optional<std::string> open_item_id;

  static KeyOutcome handled() { return KeyOutcome{true, std::nullopt}; }
  static KeyOutcome unhandled() { return KeyOutcome{false, std::nullopt}; }
  static KeyOutcome open(std::string item_id) {
    return KeyOutcome{true, std::move(item_id)};
  }
};

// One top-level, thin client view over a slice of the local-api. Each
// view owns only its own display state — no view talks to notcurses
// planes outside the `content` plane `render()` is given.
class View {
 public:
  virtual ~View() = default;

  virtual const char* title() const = 0;

  // Called whenever the view becomes active and after any action that
  // mutated server state, so the view always renders fresh data rather
  // than a stale local copy.
  virtual void refresh(ApiClient& api) = 0;

  // Renders into `plane`, already erased for this frame, spanning
  // `rows` x `cols`.
  virtual void render(ncplane* plane, unsigned rows, unsigned cols) = 0;

  // A focused text field consumes printable keys before the app's
  // global bindings, per `docs/09-terminal-ui.md`'s input model — a
  // view signals that by returning `consumed = true`.
  virtual KeyOutcome handle_key(const ncinput& input, ApiClient& api) = 0;

  // A short, one-line status/hint message the app status bar shows
  // beneath the view's own content — e.g. the last action's result.
  std::string status_message;
};

}  // namespace veloura
