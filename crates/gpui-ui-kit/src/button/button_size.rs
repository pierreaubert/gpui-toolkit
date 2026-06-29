/// Button size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    /// Extra small button
    Xs,
    /// Small button
    Sm,
    /// Medium button (default)
    #[default]
    Md,
    /// Large button
    Lg,
}

impl From<crate::ComponentSize> for ButtonSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}
