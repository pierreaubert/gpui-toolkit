use super::misc::rgba;
use gpui::*;

/// Visual settings used by the reusable spectrum axis renderers.
#[derive(Clone)]
pub struct SpectrumAxisTheme {
    pub text_color: Rgba,
    pub text_size: Rems,
    pub db_axis_width: f32,
    pub db_axis_padding_right: Rems,
    pub db_label_offset_y: f32,
    pub frequency_axis_height: f32,
    pub frequency_label_offset_x: f32,
}

impl Default for SpectrumAxisTheme {
    fn default() -> Self {
        Self {
            text_color: rgba(0xa1a1aaff),
            text_size: rems(0.75),
            db_axis_width: 32.0,
            db_axis_padding_right: rems(0.25),
            db_label_offset_y: -6.0,
            frequency_axis_height: 20.0,
            frequency_label_offset_x: -12.0,
        }
    }
}
