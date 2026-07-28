//! Veloura desktop GUI (Milestone D). Links `application` directly and
//! opens its own `AppContext`, exactly like `crates/cli` does — no
//! local-api/HTTP round trip needed for a single desktop process.

mod app;
mod screens;
mod theme;

use std::sync::Arc;

use application::AppContext;

fn main() -> iced::Result {
    let ctx = Arc::new(AppContext::open_default().expect("open default AppContext"));

    iced::application(app::App::title, app::App::update, app::App::view)
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .run_with(move || app::App::new(ctx.clone()))
}
