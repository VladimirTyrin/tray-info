use super::{Rgb, Symbol};

/// Everything that determines how one tray icon looks. Equal specs render identical icons,
/// which lets the tray skip redundant icon updates.
#[derive(Debug, Clone, PartialEq)]
pub enum IconSpec {
    /// A device symbol inside a ring filled clockwise from 12 o'clock to `fraction`.
    Ring {
        symbol: Symbol,
        symbol_color: Rgb,
        /// Filled part, `0.0..=1.0`.
        fraction: f32,
        fill: Rgb,
        track: Rgb,
    },
    /// A large number in the bottom-right corner with a small symbol in the top-left one.
    Reading {
        symbol: Symbol,
        text: String,
        color: Rgb,
    },
}
