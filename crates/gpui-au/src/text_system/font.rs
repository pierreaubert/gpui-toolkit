use font_kit::properties::{Style as FontkitStyle, Weight as FontkitWeight};
use gpui::{FontStyle, FontWeight};

pub(super) fn font_weight_to_fontkit(value: FontWeight) -> FontkitWeight {
    FontkitWeight(value.0)
}

pub(super) fn font_style_to_fontkit(style: FontStyle) -> FontkitStyle {
    match style {
        FontStyle::Normal => FontkitStyle::Normal,
        FontStyle::Italic => FontkitStyle::Italic,
        FontStyle::Oblique => FontkitStyle::Oblique,
    }
}
