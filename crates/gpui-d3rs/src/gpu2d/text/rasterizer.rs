//! Glyph rasterization using fontdue

/// Glyph rasterizer wrapper
pub struct GlyphRasterizer {
    font: fontdue::Font,
}

impl GlyphRasterizer {
    /// Create a new rasterizer from font data
    pub fn new(font_data: &[u8]) -> Self {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
            .expect("Failed to parse font");
        Self { font }
    }

    /// Rasterize a character at the given size
    ///
    /// Returns (metrics, bitmap) where bitmap is a grayscale alpha image
    pub fn rasterize(&self, c: char, size: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.font.rasterize(c, size)
    }

    /// Get metrics for a character without rasterizing
    pub fn metrics(&self, c: char, size: f32) -> fontdue::Metrics {
        self.font.metrics(c, size)
    }

    /// Calculate the width of a string at the given size
    pub fn measure_text(&self, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|c| self.font.metrics(c, size).advance_width)
            .sum()
    }
}
