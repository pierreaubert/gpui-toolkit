//! Theme editor main component
//!
//! Provides the main theme editor UI with:
//! - Color group navigation
//! - Color editing with live preview via modal
//! - Export to JSON and Rust

mod color_field;
mod misc;
mod theme_editor;
mod types;

pub use theme_editor::*;
