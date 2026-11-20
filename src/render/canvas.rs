use super::Rgb;
use super::text_mask::TextMask;
use std::f32::consts::TAU;

/// Samples per pixel side when anti-aliasing shapes; 4×4 is smooth enough at tray sizes.
const SUBSAMPLES: i32 = 4;

/// Square icon bitmap in straight-alpha BGRA, top-down rows, composited with "source over".
pub struct Canvas {
    size: i32,
    pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(size: i32) -> Self {
        Self {
            size,
            pixels: vec![0; (size * size) as usize],
        }
    }

    pub fn into_pixels(self) -> Vec<u32> {
        self.pixels
    }

    /// Paints `mask` with its top-left corner at (`left`, `top`); parts outside are clipped.
    pub fn draw_mask(&mut self, mask: &TextMask, left: i32, top: i32, color: Rgb) {
        for y in 0..mask.height {
            for x in 0..mask.width {
                let coverage = mask.coverage[(y * mask.width + x) as usize];
                self.blend(left + x, top + y, color, f32::from(coverage) / 255.0);
            }
        }
    }

    /// A ring touching the icon edges: `fraction` of it, clockwise from 12 o'clock, in `fill`,
    /// the rest in `track`. Coverage is supersampled for smooth edges.
    pub fn draw_ring(&mut self, fraction: f32, thickness: f32, fill: Rgb, track: Rgb) {
        let center = self.size as f32 / 2.0;
        let outer = center;
        let inner = outer - thickness;
        let filled_angle = fraction.clamp(0.0, 1.0) * TAU;
        let step = 1.0 / SUBSAMPLES as f32;
        let samples = (SUBSAMPLES * SUBSAMPLES) as f32;
        for y in 0..self.size {
            for x in 0..self.size {
                let mut fill_hits = 0u16;
                let mut track_hits = 0u16;
                for sy in 0..SUBSAMPLES {
                    for sx in 0..SUBSAMPLES {
                        let dx = x as f32 + (sx as f32 + 0.5) * step - center;
                        let dy = y as f32 + (sy as f32 + 0.5) * step - center;
                        let distance = dx.hypot(dy);
                        if distance < inner || distance > outer {
                            continue;
                        }
                        // Angle measured clockwise from straight up, in 0..TAU.
                        if dx.atan2(-dy).rem_euclid(TAU) < filled_angle {
                            fill_hits += 1;
                        } else {
                            track_hits += 1;
                        }
                    }
                }
                self.blend(x, y, track, f32::from(track_hits) / samples);
                self.blend(x, y, fill, f32::from(fill_hits) / samples);
            }
        }
    }

    fn blend(&mut self, x: i32, y: i32, color: Rgb, coverage: f32) {
        if coverage <= 0.0 || x < 0 || y < 0 || x >= self.size || y >= self.size {
            return;
        }
        let pixel = &mut self.pixels[(y * self.size + x) as usize];
        let [db, dg, dr, da] = pixel.to_le_bytes();
        let destination_alpha = f32::from(da) / 255.0;
        let alpha = coverage + destination_alpha * (1.0 - coverage);
        let mix = |source: u8, destination: u8| -> u8 {
            let value = (f32::from(source) * coverage
                + f32::from(destination) * destination_alpha * (1.0 - coverage))
                / alpha;
            value.round().clamp(0.0, 255.0) as u8
        };
        *pixel = u32::from_le_bytes([
            mix(color.b, db),
            mix(color.g, dg),
            mix(color.r, dr),
            (alpha * 255.0).round() as u8,
        ]);
    }
}
