/// Curve interpolation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    /// Linear interpolation (straight lines between points)
    Linear,
    /// Step function (horizontal then vertical)
    Step,
    /// Step before (vertical then horizontal)
    StepBefore,
    /// Step after (horizontal then vertical)
    StepAfter,
    /// Basis spline using the shared D3 curve interpolator.
    Basis,
    /// Cardinal spline with zero tension.
    Cardinal,
    /// Centripetal Catmull-Rom spline.
    CatmullRom,
    /// Monotone cubic interpolation in X.
    MonotoneX,
    /// Natural cubic spline.
    Natural,
}

/// Stroke dash array pattern for dashed/dotted lines.
///
/// Defines repeating on/off patterns for line rendering, similar to SVG's
/// `stroke-dasharray` attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeDashArray {
    /// Continuous stroke with no gaps.
    Solid,
    /// Dotted line: small dash, equal gap (e.g., 2px on, 2px off)
    Dotted,
    /// Dashed line: longer dash, shorter gap (e.g., 6px on, 3px off)
    Dashed,
    /// Dash-dot pattern (e.g., 6px dash, 3px gap, 2px dot, 3px gap)
    DashDot,
    /// Custom pattern: alternating on/off lengths in pixels.
    /// Must contain an even number of elements (on, off pairs).
    /// E.g., `vec![10.0, 5.0]` means 10px dash, 5px gap, repeating.
    Custom(Vec<f32>),
}
