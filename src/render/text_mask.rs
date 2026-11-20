use super::Typeface;
use crate::win::debug_log;
use anyhow::{Result, bail};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, FF_SWISS, GdiFlush, GetTextMetricsW, HGDIOBJ, OUT_TT_PRECIS, SelectObject,
    SetBkMode, SetTextAlign, SetTextColor, TA_BASELINE, TA_LEFT, TEXTMETRICW, TRANSPARENT,
    TextOutW, VARIABLE_PITCH,
};

/// GDI font request: `height` is the em height in pixels, `width` the average character width
/// (0 lets GDI keep the natural proportions; smaller values condense the glyphs).
#[derive(Debug, Clone, Copy)]
pub struct FontSize {
    pub height: i32,
    pub width: i32,
}

/// Anti-aliased coverage of rendered text, cropped to the ink bounding box.
pub struct TextMask {
    pub width: i32,
    pub height: i32,
    /// Row-major coverage, 0 (empty) to 255 (solid).
    pub coverage: Vec<u8>,
    /// Average character width GDI picked for the font, used to compute condensing.
    pub average_char_width: i32,
}

impl TextMask {
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Draws white text on black with grayscale anti-aliasing; the gray level is the coverage.
    /// GDI cannot write alpha, so this two-step approach is how we get smooth transparent text.
    pub fn render(text: &str, typeface: Typeface, font_size: FontSize) -> Result<Self> {
        let glyphs: Vec<u16> = text.encode_utf16().collect();
        let canvas_height = font_size.height * 2;
        let canvas_width = font_size.height * (glyphs.len() as i32 + 1);

        let dc = unsafe { CreateCompatibleDC(None) };
        if dc.is_invalid() {
            bail!("CreateCompatibleDC failed");
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: canvas_width,
                // Negative height makes the DIB top-down, so row 0 is the top row.
                biHeight: -canvas_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match unsafe {
            CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        } {
            Ok(bitmap) => bitmap,
            Err(error) => {
                delete_dc(dc);
                return Err(anyhow::Error::new(error).context("CreateDIBSection for text failed"));
            }
        };
        let font = unsafe {
            CreateFontW(
                -font_size.height,
                font_size.width,
                0,
                0,
                typeface.weight as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                u32::from(VARIABLE_PITCH.0) | u32::from(FF_SWISS.0),
                typeface.family,
            )
        };

        let previous_bitmap = unsafe { SelectObject(dc, bitmap.into()) };
        let previous_font = unsafe { SelectObject(dc, font.into()) };
        let mut metrics = TEXTMETRICW::default();
        unsafe {
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00FF_FFFF));
            SetTextAlign(dc, TA_BASELINE | TA_LEFT);
            let has_metrics = GetTextMetricsW(dc, &mut metrics).as_bool();
            let drawn =
                TextOutW(dc, font_size.height / 2, font_size.height * 3 / 2, &glyphs).as_bool();
            // The DIB is read directly below; pending GDI batch operations must land first.
            let flushed = GdiFlush().as_bool();
            if !(has_metrics && drawn && flushed) {
                debug_log("text rendering reported a GDI failure");
            }
        }

        let pixel_count = (canvas_width * canvas_height) as usize;
        let pixels = unsafe { std::slice::from_raw_parts(bits.cast::<u32>(), pixel_count) };
        let mask = crop(pixels, canvas_width, canvas_height, metrics.tmAveCharWidth);

        unsafe {
            SelectObject(dc, previous_font);
            SelectObject(dc, previous_bitmap);
        }
        delete_object(font.into());
        delete_object(bitmap.into());
        delete_dc(dc);
        Ok(mask)
    }
}

fn crop(pixels: &[u32], width: i32, height: i32, average_char_width: i32) -> TextMask {
    let coverage_at = |x: i32, y: i32| -> u8 {
        let [b, g, r, _] = pixels[(y * width + x) as usize].to_le_bytes();
        r.max(g).max(b)
    };
    let (mut left, mut top, mut right, mut bottom) = (width, height, -1, -1);
    for y in 0..height {
        for x in 0..width {
            if coverage_at(x, y) > 0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    if right < left {
        return TextMask {
            width: 0,
            height: 0,
            coverage: Vec::new(),
            average_char_width,
        };
    }
    let mask_width = right - left + 1;
    let mask_height = bottom - top + 1;
    let mut coverage = Vec::with_capacity((mask_width * mask_height) as usize);
    for y in top..=bottom {
        for x in left..=right {
            coverage.push(coverage_at(x, y));
        }
    }
    TextMask {
        width: mask_width,
        height: mask_height,
        coverage,
        average_char_width,
    }
}

fn delete_object(object: HGDIOBJ) {
    if !unsafe { DeleteObject(object) }.as_bool() {
        debug_log("DeleteObject failed");
    }
}

fn delete_dc(dc: windows::Win32::Graphics::Gdi::HDC) {
    if !unsafe { DeleteDC(dc) }.as_bool() {
        debug_log("DeleteDC failed");
    }
}
