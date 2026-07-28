//! Color palette transcribed from `docs/30-design-tokens.md`. Functional-
//! first: this maps the documented token values onto iced's `Theme`
//! palette (background/text/primary/success/danger), which drives every
//! built-in widget's default styling. Extra-dim/high-contrast/neutral
//! themes and the full 10-accent-token system are deferred to a polish
//! pass — see `docs/07-visual-design-system.md`.

use iced::theme::Palette;
use iced::{Color, Theme};

pub const INDIGO: Color = rgb(0x63, 0x66, 0xF1);
pub const MINT: Color = rgb(0x34, 0xD3, 0x99);
pub const RED: Color = rgb(0xEF, 0x44, 0x44);
pub const CANVAS: Color = rgb(0x00, 0x00, 0x00);
pub const TEXT_PRIMARY: Color = rgb(0xF2, 0xF0, 0xEA);

// Reserved for the pixel-accurate polish pass (docs/52-sample-ui-spec.md's
// per-component color mappings, e.g. seafoam=play, aquamarine=progress) —
// not every token is consumed by this milestone's minimal styling yet.
#[allow(dead_code)]
pub const SEAFOAM: Color = rgb(0x2D, 0xD4, 0xBF);
#[allow(dead_code)]
pub const AQUAMARINE: Color = rgb(0x22, 0xD3, 0xEE);
#[allow(dead_code)]
pub const MOONSTONE: Color = rgb(0x94, 0xA3, 0xB8);
#[allow(dead_code)]
pub const YELLOW: Color = rgb(0xFF, 0xD1, 0x66);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

pub fn dark() -> Theme {
    Theme::custom(
        "Veloura Dark".to_string(),
        Palette {
            background: CANVAS,
            text: TEXT_PRIMARY,
            primary: INDIGO,
            success: MINT,
            danger: RED,
        },
    )
}

pub fn light() -> Theme {
    Theme::custom(
        "Veloura Light".to_string(),
        Palette {
            background: Color::WHITE,
            text: Color::BLACK,
            primary: INDIGO,
            success: MINT,
            danger: RED,
        },
    )
}

pub fn from_app_theme(theme: application::Theme) -> Theme {
    match theme {
        application::Theme::Dark => dark(),
        application::Theme::Light => light(),
    }
}
