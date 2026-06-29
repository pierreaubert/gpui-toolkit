use gpui::{Pixels, px};
use gpui_design::DesignSystem;

/// Spacing values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackSpacing {
    /// No spacing
    None,
    /// Extra small (2px)
    Xs,
    /// Small (4px)
    Sm,
    /// Medium (8px, default)
    #[default]
    Md,
    /// Large (16px)
    Lg,
    /// Extra large (24px)
    Xl,
    /// 2X large (32px)
    Xxl,
    /// Custom spacing
    Custom(Pixels),
}

impl StackSpacing {
    pub(super) fn to_pixels_with_design(&self, design: &DesignSystem) -> Pixels {
        let grid = design.spacing.grid_unit;
        match self {
            StackSpacing::None => px(0.0),
            StackSpacing::Xs => px(grid * 0.5),
            StackSpacing::Sm => px(grid),
            StackSpacing::Md => px(design.spacing.control_gap),
            StackSpacing::Lg => px(design.spacing.section_gap),
            StackSpacing::Xl => px(design.spacing.section_gap + design.spacing.control_gap),
            StackSpacing::Xxl => px(design.spacing.section_gap * 2.0),
            StackSpacing::Custom(p) => *p,
        }
    }
}
