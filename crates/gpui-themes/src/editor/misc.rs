use crate::theme::BuiltInThemePreset;
use gpui::*;

/// Transparent color constant
pub(super) const TRANSPARENT: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub(super) fn editor_header_presets() -> &'static [BuiltInThemePreset] {
    BuiltInThemePreset::all()
}
