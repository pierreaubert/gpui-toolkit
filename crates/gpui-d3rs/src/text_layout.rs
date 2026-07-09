//! Renderer-independent chart text layout.

use std::error::Error;
use std::fmt;

/// Horizontal text anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAnchorX {
    /// Anchor the left edge.
    #[default]
    Start,
    /// Anchor the horizontal center.
    Middle,
    /// Anchor the right edge.
    End,
}

/// Vertical text anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAnchorY {
    /// Anchor the top edge.
    Top,
    /// Anchor the vertical center.
    Middle,
    /// Anchor the baseline.
    #[default]
    Alphabetic,
    /// Anchor the bottom edge.
    Bottom,
}

/// Checked text-layout configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayoutConfig {
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub letter_spacing: f32,
    pub max_width: Option<f32>,
    pub rotation_radians: f32,
    pub anchor_x: TextAnchorX,
    pub anchor_y: TextAnchorY,
}

impl TextLayoutConfig {
    /// Create a config with a font size and D3-style alphabetic baseline anchoring.
    pub const fn new(font_size: f32) -> Self {
        Self {
            font_size,
            line_height: None,
            letter_spacing: 0.0,
            max_width: None,
            rotation_radians: 0.0,
            anchor_x: TextAnchorX::Start,
            anchor_y: TextAnchorY::Alphabetic,
        }
    }

    /// Set explicit line height in pixels.
    pub const fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }

    /// Set letter spacing in pixels.
    pub const fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    /// Set max line width in pixels for word wrapping.
    pub const fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set text rotation in radians around the resolved anchor point.
    pub const fn rotation(mut self, rotation_radians: f32) -> Self {
        self.rotation_radians = rotation_radians;
        self
    }

    /// Set text anchors.
    pub const fn anchors(mut self, anchor_x: TextAnchorX, anchor_y: TextAnchorY) -> Self {
        self.anchor_x = anchor_x;
        self.anchor_y = anchor_y;
        self
    }

    fn resolved_line_height(self) -> f32 {
        self.line_height
            .unwrap_or_else(|| default_line_height(self.font_size))
    }
}

impl Default for TextLayoutConfig {
    fn default() -> Self {
        Self::new(12.0)
    }
}

/// Checked text-layout error.
#[derive(Debug, Clone, PartialEq)]
pub enum TextLayoutError {
    /// A numeric configuration field was not finite.
    NonFiniteConfig { field: &'static str },
    /// A numeric configuration field was not positive.
    NonPositiveConfig { field: &'static str, value: f32 },
}

impl fmt::Display for TextLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteConfig { field } => {
                write!(f, "text layout field {field} must be finite")
            }
            Self::NonPositiveConfig { field, value } => {
                write!(f, "text layout field {field} must be positive, got {value}")
            }
        }
    }
}

impl Error for TextLayoutError {}

/// Two-dimensional point in text-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextPoint {
    pub x: f32,
    pub y: f32,
}

impl TextPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Rectangle in text-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBounds {
    pub origin: TextPoint,
    pub width: f32,
    pub height: f32,
}

impl TextBounds {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: TextPoint::new(x, y),
            width,
            height,
        }
    }
}

/// One laid-out text line.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLineLayout {
    pub text: String,
    pub index: usize,
    pub baseline: TextPoint,
    pub bounds: TextBounds,
}

/// Renderer-independent chart text layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub text: String,
    pub font_size: f32,
    pub line_height: f32,
    pub width: f32,
    pub height: f32,
    pub anchor: TextPoint,
    pub unrotated_bounds: TextBounds,
    pub rotated_bounds: TextBounds,
    pub lines: Vec<TextLineLayout>,
}

impl TextLayout {
    /// Build checked text layout from a string and config.
    pub fn try_from_text(
        text: impl Into<String>,
        config: TextLayoutConfig,
    ) -> Result<Self, TextLayoutError> {
        validate_config(config)?;

        let text = text.into();
        let line_height = config.resolved_line_height();
        let lines = wrap_lines(&text, config);
        let widths = lines
            .iter()
            .map(|line| measure_text_width(line, config.font_size, config.letter_spacing))
            .collect::<Vec<_>>();
        let width = widths.iter().copied().fold(0.0_f32, f32::max);
        let height = if lines.is_empty() {
            line_height
        } else {
            line_height * lines.len() as f32
        };
        let anchor = anchor_point(width, height, config);
        let baseline_offset = baseline_offset(config.font_size, line_height);
        let line_layouts = lines
            .into_iter()
            .zip(widths)
            .enumerate()
            .map(|(index, (line, width))| {
                let y = index as f32 * line_height;
                TextLineLayout {
                    text: line,
                    index,
                    baseline: TextPoint::new(0.0, y + baseline_offset),
                    bounds: TextBounds::new(0.0, y, width, line_height),
                }
            })
            .collect::<Vec<_>>();
        let unrotated_bounds = TextBounds::new(0.0, 0.0, width, height);
        let rotated_bounds = rotated_bounds(width, height, anchor, config.rotation_radians);

        Ok(Self {
            text,
            font_size: config.font_size,
            line_height,
            width,
            height,
            anchor,
            unrotated_bounds,
            rotated_bounds,
            lines: line_layouts,
        })
    }

    /// Build text layout and panic on invalid input, matching permissive helpers.
    pub fn from_text(text: impl Into<String>, config: TextLayoutConfig) -> Self {
        Self::try_from_text(text, config).expect("invalid text layout configuration")
    }
}

/// Convenience helper for checked text layout.
pub fn text_layout(
    text: impl Into<String>,
    config: TextLayoutConfig,
) -> Result<TextLayout, TextLayoutError> {
    TextLayout::try_from_text(text, config)
}

/// Measure text width using deterministic chart-label heuristics.
pub fn measure_text_width(text: &str, font_size: f32, letter_spacing: f32) -> f32 {
    let mut width = 0.0;
    let mut visible_count = 0usize;
    for ch in text.chars() {
        if is_combining_mark(ch) {
            continue;
        }
        visible_count += 1;
        width += if ch.is_ascii() {
            font_size * 0.6
        } else if is_wide_char(ch) || is_emoji_hint(ch) {
            font_size
        } else {
            font_size * 0.65
        };
    }
    if visible_count > 1 {
        width += letter_spacing * (visible_count - 1) as f32;
    }
    width.max(0.0)
}

fn validate_config(config: TextLayoutConfig) -> Result<(), TextLayoutError> {
    for (field, value) in [
        ("font_size", config.font_size),
        ("letter_spacing", config.letter_spacing),
        ("rotation_radians", config.rotation_radians),
    ] {
        if !value.is_finite() {
            return Err(TextLayoutError::NonFiniteConfig { field });
        }
    }
    if config.font_size <= 0.0 {
        return Err(TextLayoutError::NonPositiveConfig {
            field: "font_size",
            value: config.font_size,
        });
    }
    if let Some(line_height) = config.line_height {
        if !line_height.is_finite() {
            return Err(TextLayoutError::NonFiniteConfig {
                field: "line_height",
            });
        }
        if line_height <= 0.0 {
            return Err(TextLayoutError::NonPositiveConfig {
                field: "line_height",
                value: line_height,
            });
        }
    }
    if let Some(max_width) = config.max_width {
        if !max_width.is_finite() {
            return Err(TextLayoutError::NonFiniteConfig { field: "max_width" });
        }
        if max_width <= 0.0 {
            return Err(TextLayoutError::NonPositiveConfig {
                field: "max_width",
                value: max_width,
            });
        }
    }

    Ok(())
}

fn wrap_lines(text: &str, config: TextLayoutConfig) -> Vec<String> {
    let mut lines = Vec::new();
    for source_line in text.split('\n') {
        match config.max_width {
            Some(max_width) => wrap_source_line(source_line, config, max_width, &mut lines),
            None => lines.push(source_line.to_string()),
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn wrap_source_line(
    source_line: &str,
    config: TextLayoutConfig,
    max_width: f32,
    lines: &mut Vec<String>,
) {
    let mut current = String::new();
    for word in source_line.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty()
            && measure_text_width(&candidate, config.font_size, config.letter_spacing) > max_width
        {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    lines.push(current);
}

fn anchor_point(width: f32, height: f32, config: TextLayoutConfig) -> TextPoint {
    let x = match config.anchor_x {
        TextAnchorX::Start => 0.0,
        TextAnchorX::Middle => width / 2.0,
        TextAnchorX::End => width,
    };
    let y = match config.anchor_y {
        TextAnchorY::Top => 0.0,
        TextAnchorY::Middle => height / 2.0,
        TextAnchorY::Alphabetic => baseline_offset(config.font_size, config.resolved_line_height()),
        TextAnchorY::Bottom => height,
    };
    TextPoint::new(x, y)
}

fn rotated_bounds(width: f32, height: f32, anchor: TextPoint, rotation: f32) -> TextBounds {
    let corners = [
        TextPoint::new(0.0, 0.0),
        TextPoint::new(width, 0.0),
        TextPoint::new(0.0, height),
        TextPoint::new(width, height),
    ];
    let cos_r = rotation.cos();
    let sin_r = rotation.sin();
    let mut min_x = anchor.x;
    let mut max_x = anchor.x;
    let mut min_y = anchor.y;
    let mut max_y = anchor.y;

    for corner in corners {
        let dx = corner.x - anchor.x;
        let dy = corner.y - anchor.y;
        let x = anchor.x + dx * cos_r - dy * sin_r;
        let y = anchor.y + dx * sin_r + dy * cos_r;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    TextBounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn default_line_height(font_size: f32) -> f32 {
    (font_size * 1.25).ceil().max(1.0)
}

fn baseline_offset(font_size: f32, line_height: f32) -> f32 {
    (font_size * 0.85).ceil().clamp(0.0, line_height)
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036f
            | 0x1ab0..=0x1aff
            | 0x1dc0..=0x1dff
            | 0x20d0..=0x20ff
            | 0xfe20..=0xfe2f
    )
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe19
            | 0xfe30..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
            | 0x1f300..=0x1faff
    )
}

fn is_emoji_hint(ch: char) -> bool {
    matches!(ch as u32, 0x2600..=0x27bf | 0x1f000..=0x1faff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_layout_measures_ascii_and_wide_text() {
        let ascii = measure_text_width("abc", 10.0, 1.0);
        let wide = measure_text_width("日本", 10.0, 1.0);

        assert_eq!(ascii, 20.0);
        assert_eq!(wide, 21.0);
    }

    #[test]
    fn text_layout_wraps_lines_and_sets_baselines() {
        let config = TextLayoutConfig::new(10.0)
            .line_height(14.0)
            .max_width(48.0);

        let layout = text_layout("alpha beta gamma", config).unwrap();

        assert_eq!(layout.lines.len(), 3);
        assert_eq!(layout.lines[0].text, "alpha");
        assert_eq!(layout.lines[1].baseline, TextPoint::new(0.0, 23.0));
        assert_eq!(layout.height, 42.0);
    }

    #[test]
    fn text_layout_honors_anchors_and_rotation() {
        let config = TextLayoutConfig::new(12.0)
            .anchors(TextAnchorX::Middle, TextAnchorY::Middle)
            .rotation(std::f32::consts::FRAC_PI_2);

        let layout = text_layout("abcd", config).unwrap();

        assert_eq!(
            layout.anchor,
            TextPoint::new(layout.width / 2.0, layout.height / 2.0)
        );
        assert!(layout.rotated_bounds.width > 0.0);
        assert!(layout.rotated_bounds.height > layout.unrotated_bounds.height * 0.9);
    }

    #[test]
    fn text_layout_preserves_explicit_newlines() {
        let layout = text_layout("left\nright", TextLayoutConfig::new(10.0)).unwrap();

        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.lines[0].text, "left");
        assert_eq!(layout.lines[1].text, "right");
    }

    #[test]
    fn text_layout_rejects_invalid_configuration() {
        assert_eq!(
            text_layout("bad", TextLayoutConfig::new(0.0)),
            Err(TextLayoutError::NonPositiveConfig {
                field: "font_size",
                value: 0.0
            })
        );
        assert_eq!(
            text_layout("bad", TextLayoutConfig::new(10.0).letter_spacing(f32::NAN)),
            Err(TextLayoutError::NonFiniteConfig {
                field: "letter_spacing"
            })
        );
        assert_eq!(
            text_layout("bad", TextLayoutConfig::new(10.0).max_width(-1.0)),
            Err(TextLayoutError::NonPositiveConfig {
                field: "max_width",
                value: -1.0
            })
        );
    }
}
