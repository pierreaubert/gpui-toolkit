//! Shared size definitions for UI components.
//!
//! This module provides a standardized size enum that components can use
//! for consistent sizing across the UI kit.
//!
//! # Usage
//!
//! Components can either use `ComponentSize` directly or define their own
//! size enum and implement `From<ComponentSize>` for gradual migration.
//!
//! ```rust,ignore
//! use gpui_ui_kit::ComponentSize;
//!
//! // Direct usage
//! let button = Button::new("Click me").size(ComponentSize::Md);
//!
//! // With component-specific enum (for backwards compatibility)
//! let slider = Slider::new().size(SliderSize::from(ComponentSize::Lg));
//! ```

/// Standard component sizes used across the UI kit.
///
/// The naming convention is:
/// - `Xs` - Extra small (for compact/dense UIs)
/// - `Sm` - Small
/// - `Md` - Medium (default for most components)
/// - `Lg` - Large
/// - `Xl` - Extra large (for prominent elements)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ComponentSize {
    /// Extra small - for compact/dense UIs
    Xs,
    /// Small - slightly reduced size
    Sm,
    /// Medium - default size for most components
    #[default]
    Md,
    /// Large - increased size for emphasis
    Lg,
    /// Extra large - for prominent/hero elements
    Xl,
}

impl ComponentSize {
    /// Returns a multiplier relative to the medium size.
    ///
    /// Useful for computing derived sizes (padding, margins, etc.)
    /// - Xs: 0.5
    /// - Sm: 0.75
    /// - Md: 1.0
    /// - Lg: 1.5
    /// - Xl: 2.0
    pub fn multiplier(&self) -> f32 {
        match self {
            ComponentSize::Xs => 0.5,
            ComponentSize::Sm => 0.75,
            ComponentSize::Md => 1.0,
            ComponentSize::Lg => 1.5,
            ComponentSize::Xl => 2.0,
        }
    }

    /// Returns the size as a pixel value given a base size.
    ///
    /// # Example
    /// ```rust,ignore
    /// let size = ComponentSize::Lg;
    /// let height = size.to_px(24.0); // Returns 36.0 (24 * 1.5)
    /// ```
    pub fn to_px(&self, base: f32) -> f32 {
        base * self.multiplier()
    }
}

/// Trait for components that support sizing.
///
/// Implement this trait to enable consistent size handling across components.
pub trait Sized {
    /// Set the component size.
    fn size(self, size: ComponentSize) -> Self;
}
