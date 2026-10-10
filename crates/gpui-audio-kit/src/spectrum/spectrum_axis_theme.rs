use super::misc::rgba;
use gpui::*;

/// Visual settings used by the reusable spectrum axis renderers.
#[derive(Clone)]
pub struct SpectrumAxisTheme {
    pub text_color: Rgba,
    pub tick_color: Rgba,
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
            tick_color: rgba(0x737373ff),
            text_size: rems(0.75),
            db_axis_width: 32.0,
            db_axis_padding_right: rems(0.25),
            db_label_offset_y: -6.0,
            frequency_axis_height: 20.0,
            frequency_label_offset_x: -12.0,
        }
    }
}

impl SpectrumAxisTheme {
    /// Apply platform design geometry to the axis theme (label text size, dB
    /// axis width, and axis padding). Colors and label offsets are untouched.
    ///
    /// Text sizes assume the default 16px rem root, matching the `Default`
    /// values (`rems(0.75)` = 12px).
    pub fn apply_design(&mut self, design: &gpui_design::DesignSystem) {
        self.text_size = rems(design.typography.small_size / 16.0);
        self.db_axis_width = design.interaction.min_touch_target;
        self.db_axis_padding_right = rems(design.spacing.grid_unit / 16.0);
    }
}
