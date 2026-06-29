use serde::Serialize;

/// Shadow/elevation model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ElevationRules {
    /// Level 0 (flat): shadow blur radius in px. 0 = no shadow.
    pub level_0_blur: f32,
    /// Level 1 (raised card): shadow blur radius in px.
    pub level_1_blur: f32,
    /// Level 2 (dialog/popover): shadow blur radius in px.
    pub level_2_blur: f32,
    /// Shadow opacity multiplier (0.0–1.0).
    pub shadow_opacity: f32,
    /// Shadow Y offset in px (positive = downward).
    pub shadow_y_offset: f32,
}

impl ElevationRules {
    pub fn new(
        level_0_blur: f32,
        level_1_blur: f32,
        level_2_blur: f32,
        shadow_opacity: f32,
        shadow_y_offset: f32,
    ) -> Self {
        assert!(level_0_blur >= 0.0, "level_0_blur must be >= 0");
        assert!(level_1_blur >= 0.0, "level_1_blur must be >= 0");
        assert!(level_2_blur >= 0.0, "level_2_blur must be >= 0");
        assert!(
            (0.0..=1.0).contains(&shadow_opacity),
            "shadow_opacity must be in [0, 1]"
        );
        Self {
            level_0_blur,
            level_1_blur,
            level_2_blur,
            shadow_opacity,
            shadow_y_offset,
        }
    }
}
