use serde::Serialize;

/// Touch target and interaction sizing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct InteractionRules {
    /// Minimum touch target size in px (accessibility).
    pub min_touch_target: f32,
    /// Border width for interactive elements: px.
    pub border_width: f32,
    /// Focus ring width: px.
    pub focus_ring_width: f32,
    /// Focus ring offset from element edge: px.
    pub focus_ring_offset: f32,
}

impl InteractionRules {
    pub fn new(
        min_touch_target: f32,
        border_width: f32,
        focus_ring_width: f32,
        focus_ring_offset: f32,
    ) -> Self {
        assert!(min_touch_target >= 0.0, "min_touch_target must be >= 0");
        assert!(border_width >= 0.0, "border_width must be >= 0");
        assert!(focus_ring_width >= 0.0, "focus_ring_width must be >= 0");
        assert!(focus_ring_offset >= 0.0, "focus_ring_offset must be >= 0");
        Self {
            min_touch_target,
            border_width,
            focus_ring_width,
            focus_ring_offset,
        }
    }
}
