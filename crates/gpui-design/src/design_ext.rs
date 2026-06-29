use crate::DesignSystem;

/// Extension trait for easy design system access from GPUI `App`.
#[cfg(feature = "gpui")]
pub trait DesignExt {
    fn design(&self) -> std::sync::Arc<DesignSystem>;
}
