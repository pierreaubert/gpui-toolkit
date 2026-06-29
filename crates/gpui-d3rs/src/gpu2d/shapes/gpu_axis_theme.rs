use super::super::primitives::Color4;

/// GPU-accelerated axis theme configuration
#[derive(Clone)]
pub struct GpuAxisTheme {
    /// Line color for domain line and ticks
    pub line_color: Color4,
    /// Text color for labels
    pub label_color: Color4,
}

impl Default for GpuAxisTheme {
    fn default() -> Self {
        Self {
            line_color: [1.0, 1.0, 1.0, 1.0],
            label_color: [0.9, 0.9, 0.9, 1.0],
        }
    }
}

impl GpuAxisTheme {
    /// Create with custom colors
    pub fn new(line_color: Color4, label_color: Color4) -> Self {
        Self {
            line_color,
            label_color,
        }
    }

    /// Light theme (dark text on light background)
    pub fn light() -> Self {
        Self {
            line_color: [0.2, 0.2, 0.2, 1.0],
            label_color: [0.1, 0.1, 0.1, 1.0],
        }
    }

    /// Dark theme (light text on dark background)
    pub fn dark() -> Self {
        Self::default()
    }
}
