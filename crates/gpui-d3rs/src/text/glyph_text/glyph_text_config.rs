use super::chart::chart_baseline_y;
use super::chart::chart_line_height;
use super::measure::measure_raw;
use super::types::HorizontalTextAnchor;
use super::types::LogicalTextBox;
use super::types::VerticalTextAnchor;
use super::types::horizontal_anchor_offset;
use gpui::Rgba;

#[derive(Debug, Clone)]
pub struct GlyphTextConfig {
    pub font_size: f32,
    pub color: Rgba,
    pub rotation: f32,
    pub letter_spacing: f32,
}

impl GlyphTextConfig {
    pub fn horizontal(font_size: f32, color: impl Into<Rgba>) -> Self {
        Self {
            font_size,
            color: color.into(),
            rotation: 0.0,
            letter_spacing: 0.0,
        }
    }

    /// Create horizontal text config from design-system typography defaults.
    #[cfg(feature = "gpui")]
    pub fn from_design(design: &gpui_design::DesignSystem, color: impl Into<Rgba>) -> Self {
        Self::horizontal(design.typography.base_size, color)
    }

    /// Apply design-system typography defaults.
    #[cfg(feature = "gpui")]
    pub fn with_design(mut self, design: &gpui_design::DesignSystem) -> Self {
        self.font_size = design.typography.base_size;
        self.letter_spacing = 0.0;
        self
    }

    pub fn rotated(font_size: f32, color: impl Into<Rgba>, rotation: f32) -> Self {
        Self {
            font_size,
            color: color.into(),
            rotation,
            letter_spacing: 0.0,
        }
    }

    pub fn vertical_bottom_to_top(font_size: f32, color: impl Into<Rgba>) -> Self {
        Self::rotated(font_size, color, -std::f32::consts::FRAC_PI_2)
    }

    pub fn with_letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }
}

impl Default for GlyphTextConfig {
    fn default() -> Self {
        Self::horizontal(
            12.0,
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
    }
}

pub(super) fn logical_text_box(
    text: &str,
    config: &GlyphTextConfig,
    horizontal_anchor: HorizontalTextAnchor,
    vertical_anchor: VerticalTextAnchor,
) -> LogicalTextBox {
    let raw = measure_raw(text, config.font_size, config.letter_spacing);
    let width = raw.width.max(1.0);
    let height = chart_line_height(config.font_size);
    let baseline_y = chart_baseline_y(config.font_size, height);
    let anchor = [
        horizontal_anchor_offset(width, horizontal_anchor),
        match vertical_anchor {
            VerticalTextAnchor::Top => 0.0,
            VerticalTextAnchor::Middle => height / 2.0,
            VerticalTextAnchor::Alphabetic => baseline_y,
            VerticalTextAnchor::Bottom => height,
        },
    ];

    LogicalTextBox {
        width,
        height,
        baseline_y,
        anchor,
    }
}
