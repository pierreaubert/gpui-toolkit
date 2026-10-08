//! Fixed aspect-ratio box primitive.
//!
//! [`AspectRatio`] constrains one child to a width/height ratio using
//! GPUI's native aspect-ratio support. It is a primitive for media,
//! previews, and plot surfaces.
//!
//! # Usage
//!
//! ```ignore
//! AspectRatio::new("preview")
//!     .preset(AspectRatioPreset::Widescreen)
//!     .child(thumbnail)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{AnyElement, App, Div, ElementId, Rgba, SharedString, Stateful, Window, div};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Common aspect-ratio presets as width/height factors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AspectRatioPreset {
    /// 1:1 square.
    Square,
    /// 4:3 standard.
    Standard,
    /// 16:9 widescreen (default).
    #[default]
    Widescreen,
    /// 21:9 ultrawide.
    Ultrawide,
    /// 3:4 portrait.
    Portrait,
}

impl AspectRatioPreset {
    /// Width-over-height factor for this preset.
    pub const fn ratio(self) -> f32 {
        match self {
            Self::Square => 1.0,
            Self::Standard => 4.0 / 3.0,
            Self::Widescreen => 16.0 / 9.0,
            Self::Ultrawide => 21.0 / 9.0,
            Self::Portrait => 3.0 / 4.0,
        }
    }
}

/// Theme colors for aspect-ratio box styling.
#[derive(Debug, Clone, ComponentTheme)]
pub struct AspectRatioTheme {
    /// Container background tint (transparent by default).
    #[theme(default = 0x00000000, from = transparent)]
    pub background: Rgba,
}

/// A fixed aspect-ratio box.
///
/// Constrains its child to a width/height ratio while filling the
/// available width. An empty slot renders nothing. Unlabeled boxes
/// register with an empty accessible label.
///
/// # Examples
///
/// ```ignore
/// AspectRatio::new("preview")
///     .preset(AspectRatioPreset::Widescreen)
///     .child(thumbnail)
/// ```
#[derive(IntoElement)]
pub struct AspectRatio {
    id: ElementId,
    child: Option<AnyElement>,
    ratio: f32,
    theme: Option<AspectRatioTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl AspectRatio {
    /// Create a new 16:9 aspect-ratio box.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child: None,
            ratio: AspectRatioPreset::Widescreen.ratio(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the boxed child element.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Set a custom width-over-height ratio (clamped to stay positive).
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.max(f32::MIN_POSITIVE);
        self
    }

    /// Set the ratio from a preset.
    pub fn preset(mut self, preset: AspectRatioPreset) -> Self {
        self.ratio = preset.ratio();
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: AspectRatioTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit accessible label for the ratio box group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default accessible role (`Group`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the box into a `Stateful<Div>` for further composition.
    ///
    /// Note: This bypasses accessibility registration. Prefer using the
    /// component directly via `RenderOnce` for automatic accessibility
    /// tree integration.
    pub fn build(self) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        let mut this = self;
        let theme = this.theme.take().unwrap_or_default();
        this.build_with_theme_and_design(&theme, &design)
    }

    /// Build with explicit theme and design defaults.
    pub fn build_with_theme_and_design(
        self,
        theme: &AspectRatioTheme,
        _design: &DesignSystem,
    ) -> Stateful<Div> {
        div()
            .id(self.id)
            .w_full()
            .aspect_ratio(self.ratio.max(f32::MIN_POSITIVE))
            .overflow_hidden()
            .bg(theme.background)
            .children(self.child)
    }
}

impl RenderOnce for AspectRatio {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let design = crate::design::resolve_design(self.design.clone(), cx);
        let global_theme = cx.theme();
        let theme = self
            .theme
            .take()
            .unwrap_or_else(|| AspectRatioTheme::from(global_theme.as_ref()));
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_widescreen_with_empty_slot() {
        let ratio_box = AspectRatio::new("ratio");
        assert!((ratio_box.ratio - 16.0 / 9.0).abs() < f32::EPSILON);
        assert!(ratio_box.child.is_none());
    }

    #[test]
    fn presets_resolve_to_expected_factors() {
        assert!((AspectRatioPreset::Square.ratio() - 1.0).abs() < f32::EPSILON);
        assert!((AspectRatioPreset::Standard.ratio() - 4.0 / 3.0).abs() < f32::EPSILON);
        assert!((AspectRatioPreset::Widescreen.ratio() - 16.0 / 9.0).abs() < f32::EPSILON);
        assert!((AspectRatioPreset::Ultrawide.ratio() - 21.0 / 9.0).abs() < f32::EPSILON);
        assert!((AspectRatioPreset::Portrait.ratio() - 3.0 / 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn builders_set_fields() {
        let ratio_box = AspectRatio::new("ratio")
            .child(div())
            .preset(AspectRatioPreset::Square)
            .aria_label("Preview")
            .aria_role(AriaRole::Img);
        assert!(ratio_box.child.is_some());
        assert!((ratio_box.ratio - 1.0).abs() < f32::EPSILON);
        assert_eq!(ratio_box.aria_label, Some(SharedString::from("Preview")));
        assert_eq!(ratio_box.aria_role, Some(AriaRole::Img));
    }

    #[test]
    fn ratios_clamp_to_positive() {
        assert!(AspectRatio::new("zero").ratio(0.0).ratio > 0.0);
        assert!(AspectRatio::new("negative").ratio(-2.0).ratio > 0.0);
        assert!(AspectRatio::new("nan").ratio(f32::NAN).ratio > 0.0);
    }

    #[test]
    fn build_does_not_panic() {
        let _el = AspectRatio::new("empty-ratio").build();
        let _el = AspectRatio::new("filled-ratio")
            .preset(AspectRatioPreset::Portrait)
            .child(div())
            .build();
    }
}
