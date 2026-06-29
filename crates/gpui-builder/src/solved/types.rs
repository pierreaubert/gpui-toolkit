use crate::types::Axis;

/// Warning categories emitted by [`LayoutDebugReport`].
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutDebugWarningKind<'a> {
    /// A node has a negative, NaN, or infinite size.
    InvalidSize { width: f32, height: f32 },
    /// A node is hidden but has no collapse label explaining how it is restored.
    InvisibleWithoutCollapseLabel,
    /// Visible children exceed their parent's available main-axis size.
    MainAxisOverflow {
        axis: Axis,
        used: f32,
        available: f32,
    },
    /// A visible child exceeds its parent's cross-axis size.
    CrossAxisOverflow {
        axis: Axis,
        child_id: &'a str,
        used: f32,
        available: f32,
    },
}
