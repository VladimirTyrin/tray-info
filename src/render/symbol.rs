/// Pictogram telling what an icon shows; glyphs come from the Segoe Fluent Icons font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Headphones,
    Mouse,
    Keyboard,
    Gamepad,
    Bluetooth,
    Thermometer,
    Lightning,
}

impl Symbol {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Headphones => "\u{E7F6}",
            Self::Mouse => "\u{E962}",
            Self::Keyboard => "\u{E765}",
            Self::Gamepad => "\u{E7FC}",
            Self::Bluetooth => "\u{E702}",
            Self::Thermometer => "\u{E9CA}",
            Self::Lightning => "\u{E945}",
        }
    }
}
