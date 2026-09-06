//! Glyph text rendering helpers for chart labels.

mod glyph_text;
pub mod vector_font;

pub use glyph_text::{
    GlyphTextConfig, GlyphTextMetrics, HorizontalTextAnchor, VerticalTextAnchor,
    measure_glyph_text, measure_glyph_text_width, paint_chart_text_at, paint_glyph_text_at,
    render_glyph_text, render_glyph_text_anchored,
};
pub use vector_font::{
    VectorFontConfig, measure_text_width, paint_vector_text_at, render_vector_text,
};
