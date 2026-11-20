//! The executable's icon (`assets/tray-info.ico`), drawn with the same painter as the tray icons.
//! Regenerate after changing the look with
//! `cargo test --lib regenerate_app_icon -- --ignored`; `build.rs` embeds the file.

use super::Rgb;
use super::canvas::Canvas;
use super::icon_painter::fit_symbol;

/// Sizes Explorer and the taskbar ask for at 100–200% scaling. 256 px is left out: stored
/// uncompressed it would double the executable.
const SIZES: [i32; 8] = [16, 20, 24, 32, 40, 48, 64, 128];
/// "Speed high" in Segoe Fluent Icons: a gauge, standing for "readings".
const GAUGE_GLYPH: &str = "\u{EC4A}";
const TILE: Rgb = Rgb::hex(0x1C2A3D);
const RING_FILL: Rgb = Rgb::hex(0x7CD2FF);
const RING_TRACK: Rgb = Rgb::hex(0x505A66);
const SYMBOL: Rgb = Rgb::hex(0xFFFFFF);
/// How much of the ring is filled.
const GAUGE_LEVEL: f32 = 0.66;

/// A gauge in a two-thirds filled ring on a rounded navy tile.
fn render(size: i32) -> Vec<u32> {
    let mut canvas = Canvas::new(size);
    canvas.fill_rounded_square(size as f32 * 0.22, TILE);
    let inner = size * 3 / 4;
    let mut gauge = Canvas::new(inner);
    gauge.draw_ring(
        GAUGE_LEVEL,
        (inner as f32 / 12.0).max(1.5),
        RING_FILL,
        RING_TRACK,
    );
    let glyph = fit_symbol(GAUGE_GLYPH, inner * 13 / 24, inner * 13 / 24).expect("gauge glyph");
    gauge.draw_mask(
        &glyph,
        (inner - glyph.width) / 2,
        (inner - glyph.height) / 2,
        SYMBOL,
    );
    let offset = (size - inner) / 2;
    canvas.draw_canvas(&gauge, offset, offset);
    canvas.into_pixels()
}

/// An `.ico` file with 32-bit BMP images (straight alpha, as icons expect).
fn ico(images: &[(i32, Vec<u32>)]) -> Vec<u8> {
    const DIRECTORY: usize = 6;
    const ENTRY: usize = 16;
    let mut file = Vec::new();
    file.extend_from_slice(&0u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = DIRECTORY + ENTRY * images.len();
    let mut bodies = Vec::new();
    for (size, pixels) in images {
        let body = bmp_icon_image(*size, pixels);
        // 256 would be stored as 0; larger sizes are not produced.
        file.extend_from_slice(&[*size as u8, *size as u8, 0, 0]);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&(body.len() as u32).to_le_bytes());
        file.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += body.len();
        bodies.push(body);
    }
    for body in bodies {
        file.extend_from_slice(&body);
    }
    file
}

/// BITMAPINFOHEADER (height doubled for the AND mask), bottom-up BGRA rows, empty AND mask.
fn bmp_icon_image(size: i32, pixels: &[u32]) -> Vec<u8> {
    let mask_row = (size as usize).div_ceil(32) * 4;
    let mut body = Vec::new();
    body.extend_from_slice(&40u32.to_le_bytes());
    body.extend_from_slice(&size.to_le_bytes());
    body.extend_from_slice(&(size * 2).to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&32u16.to_le_bytes());
    body.extend_from_slice(&[0; 24]);
    for y in (0..size).rev() {
        for x in 0..size {
            body.extend_from_slice(&pixels[(y * size + x) as usize].to_le_bytes());
        }
    }
    body.extend(std::iter::repeat_n(0u8, mask_row * size as usize));
    body
}

#[test]
#[ignore = "writes assets/tray-info.ico; run after changing the icon look"]
fn regenerate_app_icon() {
    let images: Vec<(i32, Vec<u32>)> = SIZES.iter().map(|&size| (size, render(size))).collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tray-info.ico");
    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")).expect("assets dir");
    std::fs::write(path, ico(&images)).expect("write icon");
}

#[test]
fn icon_is_a_tile_with_a_ring_and_symbol() {
    let size = 48;
    let pixels = render(size);
    let color_at = |x: i32, y: i32| {
        let [b, g, r, a] = pixels[(y * size + x) as usize].to_le_bytes();
        (Rgb { r, g, b }, a)
    };

    // Rounded corner stays transparent, the tile edge is opaque navy.
    assert_eq!(color_at(0, 0).1, 0);
    assert_eq!(color_at(size / 2, 1), (TILE, 255));
    // Some ink of each part is present.
    let has = |color: Rgb| {
        pixels.iter().any(|p| {
            let [b, g, r, a] = p.to_le_bytes();
            a == 255 && Rgb { r, g, b } == color
        })
    };
    assert!(has(RING_FILL) && has(RING_TRACK) && has(SYMBOL));
}
