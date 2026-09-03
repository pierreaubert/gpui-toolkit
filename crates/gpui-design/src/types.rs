use super::design_token::DesignToken;
use serde::{Deserialize, Serialize};

/// Corner radius rendering strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CornerRadiusStyle {
    /// Apple continuous corners (squircle). Renderer should use smooth curves.
    Continuous,
    /// Standard circular arcs (CSS `border-radius`).
    Circular,
}

/// Toggle control visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToggleVariant {
    /// iOS-style capsule slider with thumb.
    Capsule,
    /// Thumb rides on a visible track (Material).
    ThumbOnTrack,
    /// Segmented [OFF|ON] button pair.
    Segmented,
    /// Pill-shaped toggle (Fluent).
    Pill,
}

/// Where labels appear relative to their control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelPosition {
    /// Label below the control (Apple, Material).
    Below,
    /// Label to the right of the control (Fluent, compact UIs).
    Right,
}

/// Visual style for grouping controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupSeparatorStyle {
    /// Subtle hairline divider (Apple).
    Divider,
    /// Distinct card surface with shadow/elevation (Material).
    Card,
    /// Thin border outline (Fluent).
    Border,
    /// No visual separator — spacing only.
    None,
}

/// Accessibility and platform-conformance finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConformanceFinding {
    pub id: &'static str,
    pub message: std::borrow::Cow<'static, str>,
}

/// Style Dictionary token export for one preset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignTokenPresetExport {
    pub preset_id: &'static str,
    pub tokens: Vec<DesignToken>,
}

/// Density tier for spacing (M3 compact/medium/expanded parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DensityTier {
    /// Compact: reduced padding/gaps (e.g. Fluent, Breeze, Carbon).
    Compact,
    /// Medium: standard density (Neutral, Apple HIG, Material 3, Adwaita).
    Medium,
    /// Expanded: roomy touch-first layouts.
    Expanded,
}

impl DensityTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Medium => "medium",
            Self::Expanded => "expanded",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "compact" => Some(Self::Compact),
            "medium" | "regular" | "standard" => Some(Self::Medium),
            "expanded" | "comfortable" => Some(Self::Expanded),
            _ => None,
        }
    }

    /// Multiplier applied to control padding/gaps at this density.
    pub fn scale(&self) -> f32 {
        match self {
            Self::Compact => 0.9,
            Self::Medium => 1.0,
            Self::Expanded => 1.1,
        }
    }
}

/// Motion settings after reduced-motion policy is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MotionSpec {
    pub duration_ms: u32,
    pub fast_ms: u32,
    pub slow_ms: u32,
    pub prefer_spring: bool,
    pub reduced_motion: bool,
}
