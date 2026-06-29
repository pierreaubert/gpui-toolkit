/// Input size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputSize {
    /// Extra small input
    Xs,
    /// Small input
    Sm,
    /// Medium input (default)
    #[default]
    Md,
    /// Large input
    Lg,
}

impl From<crate::ComponentSize> for InputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputSize;
    use crate::ComponentSize;

    #[test]
    fn input_size_from_component_size() {
        assert_eq!(InputSize::from(ComponentSize::Xs), InputSize::Xs);
        assert_eq!(InputSize::from(ComponentSize::Sm), InputSize::Sm);
        assert_eq!(InputSize::from(ComponentSize::Md), InputSize::Md);
        assert_eq!(InputSize::from(ComponentSize::Lg), InputSize::Lg);
        assert_eq!(InputSize::from(ComponentSize::Xl), InputSize::Lg);
    }
}
