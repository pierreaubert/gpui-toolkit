use super::format::format_max;
use crate::util::format_number;
use std::fmt;

/// Owned sizing metadata for inspection and export.
#[derive(Debug, Clone, PartialEq)]
pub enum SizingInspection {
    /// Fixed pixel size.
    Fixed { size: f32 },
    /// Fractional size with initial ratio and bounds.
    Fractional { initial: f32, min: f32, max: f32 },
    /// Flex size with minimum size and weight.
    Flex { min: f32, weight: f32 },
    /// Text-measured size metadata. The measure implementation is intentionally
    /// not exported because it is not inspectable data.
    Text {
        text: String,
        line_height: f32,
        min: f32,
    },
}

impl SizingInspection {
    /// Return a compact, stable summary.
    pub fn summary(&self) -> String {
        match self {
            Self::Fixed { size } => format!("fixed({})", format_number(*size)),
            Self::Fractional { initial, min, max } => format!(
                "fractional(initial={}, min={}, max={})",
                format_number(*initial),
                format_number(*min),
                format_max(*max)
            ),
            Self::Flex { min, weight } => format!(
                "flex(min={}, weight={})",
                format_number(*min),
                format_number(*weight)
            ),
            Self::Text {
                text,
                line_height,
                min,
            } => format!(
                "text(text={text:?}, line_height={}, min={})",
                format_number(*line_height),
                format_number(*min)
            ),
        }
    }
}

impl fmt::Display for SizingInspection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary())
    }
}
