use gpui_design::DesignSystem;

/// Slider size variants
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SliderSize {
    /// Small size
    Sm,
    /// Medium size (default)
    #[default]
    Md,
    /// Large size
    Lg,
}

impl SliderSize {
    pub(super) fn track_height_with_design(&self, design: &DesignSystem) -> f32 {
        match self {
            Self::Sm => design.spacing.grid_unit,
            Self::Md => design.spacing.grid_unit * 1.5,
            Self::Lg => design.spacing.grid_unit * 2.0,
        }
    }

    pub(super) fn thumb_size_with_design(&self, design: &DesignSystem) -> f32 {
        match self {
            Self::Sm => design.interaction.min_touch_target * 0.4375,
            Self::Md => design.interaction.min_touch_target * 0.5625,
            Self::Lg => design.interaction.min_touch_target * 0.6875,
        }
    }
}

impl From<crate::ComponentSize> for SliderSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}
