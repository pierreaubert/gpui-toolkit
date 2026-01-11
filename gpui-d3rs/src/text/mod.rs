//! Vector font text rendering
//!
//! This module provides a simple stroke-based vector font that can be used
//! to render text as paths, allowing for rotation and other transformations.

mod vector_font;

pub use vector_font::{
    VectorFontConfig, measure_text_width, paint_vector_text_at, render_vector_text,
};
