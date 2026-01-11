//! GPUI Themes - Theme editor and management for GPUI applications
//!
//! This crate provides:
//! - Serializable theme types with JSON and Rust code export
//! - A color picker component for editing colors (re-exported from gpui-ui-kit)
//! - A component showcase for previewing theme changes
//! - A theme editor application

mod editor;
mod showcase;
mod theme;

// Re-export ColorPickerView from gpui-ui-kit
pub use gpui_ui_kit::{ColorPickerMode, ColorPickerView};

pub use editor::ThemeEditor;
pub use showcase::ComponentShowcase;
pub use theme::{
    Color, ColorGroup, EQCurveColors, EditorTheme, GraphColors, MeterColors, PluginColors,
    SpectrumColors,
};
