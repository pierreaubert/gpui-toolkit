use super::consts::FONT;
use super::glyph::glyph_advance;
use super::glyph::glyph_left;
use super::glyph::glyph_top;
use super::is::heuristic_text_width;
use super::types::GlyphTextMetrics;
use super::types::RawMetrics;

pub fn measure_glyph_text_width(text: &str, font_size: f32) -> f32 {
    measure_glyph_text(text, font_size).width
}

pub fn measure_glyph_text(text: &str, font_size: f32) -> GlyphTextMetrics {
    let raw = measure_raw(text, font_size, 0.0);
    let height = (raw.max_y - raw.min_y).max(font_size);
    GlyphTextMetrics {
        width: raw.width,
        height,
        ascent: (-raw.min_y).max(0.0),
        descent: raw.max_y.max(0.0),
    }
}

pub(super) fn measure_raw(text: &str, font_size: f32, letter_spacing: f32) -> RawMetrics {
    if text.is_empty() {
        return RawMetrics {
            width: 0.0,
            ink_min_x: 0.0,
            ink_max_x: 0.0,
            min_y: -font_size * 0.8,
            max_y: font_size * 0.2,
        };
    }

    let mut width = 0.0;
    let mut ink_min_x = f32::INFINITY;
    let mut ink_max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let char_count = text.chars().count();

    for (idx, c) in text.chars().enumerate() {
        let metrics = FONT.metrics(c, font_size);
        let glyph_x = glyph_left(width, &metrics);
        let glyph_y = glyph_top(&metrics);

        if metrics.width > 0 && metrics.height > 0 {
            ink_min_x = ink_min_x.min(glyph_x);
            ink_max_x = ink_max_x.max(glyph_x + metrics.width as f32);
            min_y = min_y.min(glyph_y);
            max_y = max_y.max(glyph_y + metrics.height as f32);
        }

        width += glyph_advance(&metrics);
        if idx + 1 < char_count {
            width += letter_spacing;
        }
    }

    let heuristic_width = heuristic_text_width(text, font_size, letter_spacing);
    width = width.max(heuristic_width);

    if !ink_min_x.is_finite() || !ink_max_x.is_finite() || ink_min_x >= ink_max_x {
        ink_min_x = 0.0;
        ink_max_x = width.max(1.0);
    } else {
        ink_max_x = ink_max_x.max(width);
    }

    if !min_y.is_finite() || !max_y.is_finite() || min_y >= max_y {
        min_y = -font_size * 0.8;
        max_y = font_size * 0.2;
    }

    RawMetrics {
        width,
        ink_min_x,
        ink_max_x,
        min_y,
        max_y,
    }
}
