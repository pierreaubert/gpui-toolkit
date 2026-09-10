use gpui::{Rems, rems};

/// Number input size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberInputSize {
    /// Extra small size
    Xs,
    /// Small size
    Sm,
    /// Medium size (default)
    #[default]
    Md,
    /// Large size
    Lg,
}

impl From<crate::ComponentSize> for NumberInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

impl NumberInputSize {
    pub(super) fn height(&self) -> f32 {
        match self {
            Self::Xs => 20.0,
            Self::Sm => 24.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
        }
    }

    pub(super) fn button_width(&self) -> f32 {
        match self {
            Self::Xs => 16.0,
            Self::Sm => 20.0,
            Self::Md => 28.0,
            Self::Lg => 36.0,
        }
    }

    pub(super) fn font_size(&self) -> f32 {
        match self {
            Self::Xs => 10.0,
            Self::Sm => 11.0,
            Self::Md => 13.0,
            Self::Lg => 15.0,
        }
    }

    pub(super) fn padding(&self) -> f32 {
        match self {
            Self::Xs => 2.0,
            Self::Sm => 4.0,
            Self::Md => 8.0,
            Self::Lg => 12.0,
        }
    }

    /// Rem equivalents of [`Self::height`], [`Self::button_width`],
    /// [`Self::font_size`], and [`Self::padding`] (divided by the default
    /// 16 px rem). Pixel-identical at 1x zoom and scaling with font zoom via
    /// `window.set_rem_size()`.
    pub(super) fn height_rems(&self) -> Rems {
        rems(self.height() / 16.0)
    }

    pub(super) fn button_width_rems(&self) -> Rems {
        rems(self.button_width() / 16.0)
    }

    pub(super) fn font_size_rems(&self) -> Rems {
        rems(self.font_size() / 16.0)
    }

    pub(super) fn padding_rems(&self) -> Rems {
        rems(self.padding() / 16.0)
    }
}

#[cfg(test)]
mod tests {
    use super::NumberInputSize;
    use crate::ComponentSize;

    #[test]
    fn number_input_size_from_component_size() {
        assert_eq!(
            NumberInputSize::from(ComponentSize::Xs),
            NumberInputSize::Xs
        );
        assert_eq!(
            NumberInputSize::from(ComponentSize::Sm),
            NumberInputSize::Sm
        );
        assert_eq!(
            NumberInputSize::from(ComponentSize::Md),
            NumberInputSize::Md
        );
        assert_eq!(
            NumberInputSize::from(ComponentSize::Lg),
            NumberInputSize::Lg
        );
        assert_eq!(
            NumberInputSize::from(ComponentSize::Xl),
            NumberInputSize::Lg
        );
    }

    #[test]
    fn number_input_size_measurements() {
        assert_eq!(NumberInputSize::Xs.height(), 20.0);
        assert_eq!(NumberInputSize::Md.button_width(), 28.0);
        assert_eq!(NumberInputSize::Lg.font_size(), 15.0);
        assert_eq!(NumberInputSize::Sm.padding(), 4.0);
    }

    #[test]
    fn number_input_size_rems_match_px_at_default_rem() {
        // Default GPUI rem is 16 px: rem values must reproduce the px table.
        assert_eq!(NumberInputSize::Md.height_rems().0, 32.0 / 16.0);
        assert_eq!(NumberInputSize::Md.button_width_rems().0, 28.0 / 16.0);
        assert_eq!(NumberInputSize::Md.font_size_rems().0, 13.0 / 16.0);
        assert_eq!(NumberInputSize::Md.padding_rems().0, 8.0 / 16.0);
    }
}
