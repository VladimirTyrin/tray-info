use anyhow::{Context, Result};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};

use super::canvas::Canvas;
use super::text_mask::{FontSize, TextMask};
use super::{IconSpec, OwnedIcon, Rgb, Typeface};
use crate::win::debug_log;

/// Digit height is calibrated on this glyph so every icon shows digits of the same size,
/// and short glyphs like the "--" placeholder are not blown up to full height.
const REFERENCE_GLYPH: &str = "0";

/// Renders the spec into `size`×`size` BGRA pixels (straight alpha, top-down rows).
pub fn rasterize(spec: &IconSpec, size: i32) -> Result<Vec<u32>> {
    let mut canvas = Canvas::new(size);
    match spec {
        IconSpec::Ring {
            symbol,
            symbol_color,
            fraction,
            fill,
            track,
        } => {
            // Tuned at 24 px (150% scaling): 2 px ring around a 13 px symbol.
            let thickness = (size as f32 / 12.0).max(1.5);
            let glyph = fit_symbol(symbol.glyph(), size * 13 / 24, size * 13 / 24)?;
            canvas.draw_ring(*fraction, thickness, *fill, *track);
            draw_centered(&mut canvas, &glyph, size, *symbol_color);
        }
        IconSpec::Reading {
            symbol,
            text,
            color,
        } => {
            // Tuned at 24 px: an 11 px symbol above 13 px digits, so the two never share a row
            // and wide numbers do not run into the symbol.
            let symbol_height = size * 11 / 24;
            let glyph = fit_symbol(symbol.glyph(), symbol_height, symbol_height)?;
            canvas.draw_mask(&glyph, 0, 0, *color);
            let digit_height = size - symbol_height;
            let digits = fit_text(text, Typeface::NUMBERS, size, digit_height)?;
            // Centred within the digit band so the "--" placeholder sits where digits would.
            canvas.draw_mask(
                &digits,
                size - digits.width,
                size - digit_height + (digit_height - digits.height) / 2,
                *color,
            );
        }
    }
    Ok(canvas.into_pixels())
}

fn draw_centered(canvas: &mut Canvas, mask: &TextMask, size: i32, color: Rgb) {
    canvas.draw_mask(
        mask,
        (size - mask.width) / 2,
        (size - mask.height) / 2,
        color,
    );
}

/// Builds a tray-ready `HICON` for the spec.
pub fn paint_icon(spec: &IconSpec, size: i32) -> Result<OwnedIcon> {
    let pixels = rasterize(spec, size)?;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let color = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }
        .context("CreateDIBSection for icon failed")?;
    unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len()) };

    // With a 32-bit colour bitmap Windows takes transparency from alpha; the AND mask only has
    // to exist. Monochrome rows are padded to 16 bits.
    let mask_bits = vec![0u8; (size as usize).div_ceil(16) * 2 * size as usize];
    let mask = unsafe { CreateBitmap(size, size, 1, 1, Some(mask_bits.as_ptr().cast())) };

    let icon_info = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: color,
    };
    // CreateIconIndirect copies both bitmaps, so they are released regardless of the outcome.
    let icon = unsafe { CreateIconIndirect(&icon_info) };
    delete_object(mask.into());
    delete_object(color.into());
    Ok(OwnedIcon::new(icon.context("CreateIconIndirect failed")?))
}

/// Scales a symbol glyph to the largest size whose ink fits the box.
fn fit_symbol(glyph: &str, max_width: i32, max_height: i32) -> Result<TextMask> {
    // Ink grows linearly with the em height, so one probe gives the right scale.
    let probe_height = max_height * 2;
    let probe = TextMask::render(
        glyph,
        Typeface::SYMBOLS,
        FontSize {
            height: probe_height,
            width: 0,
        },
    )?;
    if probe.is_empty() {
        // Symbol font missing: show the number alone rather than fail the icon.
        return Ok(probe);
    }
    let scale =
        (max_width as f32 / probe.width as f32).min(max_height as f32 / probe.height as f32);
    let font = FontSize {
        height: ((probe_height as f32 * scale).round() as i32).max(1),
        width: 0,
    };
    render_within(glyph, Typeface::SYMBOLS, font, max_width, max_height)
}

/// Scales text so a digit's ink is `max_height` tall, condensing it if it is too wide.
fn fit_text(text: &str, typeface: Typeface, max_width: i32, max_height: i32) -> Result<TextMask> {
    let probe_height = max_height * 3 / 2;
    let probe = TextMask::render(
        REFERENCE_GLYPH,
        typeface,
        FontSize {
            height: probe_height,
            width: 0,
        },
    )?;
    let mut font = FontSize {
        height: if probe.is_empty() {
            max_height
        } else {
            probe_height * max_height / probe.height
        },
        width: 0,
    };
    let mask = TextMask::render(text, typeface, font)?;
    if mask.width > max_width {
        // Too wide (e.g. "100" on a small icon): condense rather than shrink the digits.
        font.width = (mask.average_char_width * max_width / mask.width).max(1);
    }
    render_within(text, typeface, font, max_width, max_height)
}

fn render_within(
    text: &str,
    typeface: Typeface,
    mut font: FontSize,
    max_width: i32,
    max_height: i32,
) -> Result<TextMask> {
    let mut mask = TextMask::render(text, typeface, font)?;
    // Pixel rounding can still overshoot by a pixel or two.
    while (mask.width > max_width || mask.height > max_height) && font.height > 4 {
        font.height -= 1;
        if font.width > 1 {
            font.width -= 1;
        }
        mask = TextMask::render(text, typeface, font)?;
    }
    Ok(mask)
}

fn delete_object(object: HGDIOBJ) {
    if !unsafe { DeleteObject(object) }.as_bool() {
        debug_log("DeleteObject failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Symbol;

    const SIZE: i32 = 24;
    const SYMBOL_COLOR: Rgb = Rgb::hex(0xFF0000);
    const FILL: Rgb = Rgb::hex(0x00FF00);
    const TRACK: Rgb = Rgb::hex(0x0000FF);

    fn ring(fraction: f32) -> IconSpec {
        IconSpec::Ring {
            symbol: Symbol::Mouse,
            symbol_color: SYMBOL_COLOR,
            fraction,
            fill: FILL,
            track: TRACK,
        }
    }

    /// (x, y) of pixels painted exactly in `color`.
    fn painted(pixels: &[u32], color: Rgb) -> Vec<(i32, i32)> {
        pixels
            .iter()
            .enumerate()
            .filter(|&(_, &pixel)| {
                let [b, g, r, a] = pixel.to_le_bytes();
                a > 0 && Rgb { r, g, b } == color
            })
            .map(|(index, _)| (index as i32 % SIZE, index as i32 / SIZE))
            .collect()
    }

    #[test]
    fn half_full_ring_fills_the_right_half() {
        let pixels = rasterize(&ring(0.5), SIZE).expect("rasterize");

        let filled = painted(&pixels, FILL);
        let track = painted(&pixels, TRACK);
        assert!(!filled.is_empty() && !track.is_empty());
        assert!(filled.iter().all(|&(x, _)| x >= SIZE / 2));
        assert!(track.iter().all(|&(x, _)| x < SIZE / 2));
    }

    #[test]
    fn empty_ring_is_all_track() {
        let pixels = rasterize(&ring(0.0), SIZE).expect("rasterize");

        assert!(painted(&pixels, FILL).is_empty());
        assert!(!painted(&pixels, TRACK).is_empty());
    }

    #[test]
    fn symbol_sits_inside_the_ring() {
        let pixels = rasterize(&ring(1.0), SIZE).expect("rasterize");

        let symbol = painted(&pixels, SYMBOL_COLOR);
        assert!(!symbol.is_empty());
        let inside = |v: i32| (3..SIZE - 3).contains(&v);
        assert!(symbol.iter().all(|&(x, y)| inside(x) && inside(y)));
    }

    fn reading(text: &str) -> IconSpec {
        IconSpec::Reading {
            symbol: Symbol::Thermometer,
            text: text.to_owned(),
            color: FILL,
        }
    }

    #[test]
    fn reading_puts_symbol_top_left_and_number_bottom_right() {
        let pixels = rasterize(&reading("72"), SIZE).expect("rasterize");

        let ink = painted(&pixels, FILL);
        let top_left = ink.iter().any(|&(x, y)| x < SIZE / 2 && y < SIZE / 3);
        let bottom_right = ink
            .iter()
            .any(|&(x, y)| x >= SIZE * 2 / 3 && y >= SIZE * 2 / 3);
        let bottom_left_corner = ink.iter().any(|&(x, y)| x < 2 && y >= SIZE - 2);
        assert!(top_left && bottom_right && !bottom_left_corner);
    }

    #[test]
    fn three_digit_reading_fits_the_icon() {
        let pixels = rasterize(&reading("115"), SIZE).expect("rasterize");

        let ink = painted(&pixels, FILL);
        assert!(ink.iter().any(|&(x, y)| x < SIZE / 3 && y >= SIZE / 2));
        assert!(ink.iter().any(|&(x, _)| x == SIZE - 1));
    }

    #[test]
    fn creates_icon_handle() {
        let icon = paint_icon(&ring(0.8), SIZE).expect("paint");

        assert!(!icon.handle().is_invalid());
    }
}
