use windows::Win32::Graphics::Gdi::FW_NORMAL;
use windows::core::{PCWSTR, w};

/// GDI font family and weight used to draw one part of an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Typeface {
    pub family: PCWSTR,
    pub weight: u32,
}

impl Typeface {
    /// Tahoma, the font Core Temp uses for its tray digits: hinted, crisp one-pixel strokes at
    /// tray sizes. Three-digit values are condensed to fit.
    pub const NUMBERS: Self = Self {
        family: w!("Tahoma"),
        weight: FW_NORMAL.0,
    };
    /// Pictograms shipped with Windows 11; the set the shell uses for its own icons.
    pub const SYMBOLS: Self = Self {
        family: w!("Segoe Fluent Icons"),
        weight: FW_NORMAL.0,
    };
}
