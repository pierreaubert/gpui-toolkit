//! `Skeleton` component
//!
//! A content placeholder shown while data loads. Unlike the spinner-based
//! `LoadingOverlay`, the skeleton reserves the shape of incoming content
//! (text line, rectangle, or circle) with a shimmer highlight so layout
//! does not shift when real content arrives.
//!
//! # Usage
//!
//! ```ignore
//! Skeleton::new("profile-loading")
//!     .variant(SkeletonVariant::Circular)
//!     .size(SkeletonSize::Lg)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for the skeleton placeholder.
#[derive(Debug, Clone, ComponentTheme)]
pub struct SkeletonTheme {
    /// Placeholder base color.
    #[theme(default = 0x2e2e2eff, from = muted)]
    pub base: Rgba,
    /// Shimmer highlight color.
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub shimmer: Rgba,
}

/// Skeleton shape variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkeletonVariant {
    /// Full-width text line (default).
    #[default]
    Text,
    /// Fixed rectangle for media or cards.
    Rectangular,
    /// Circle for avatars.
    Circular,
}

/// Skeleton size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkeletonSize {
    /// Compact placeholder.
    Sm,
    /// Standard placeholder (default).
    #[default]
    Md,
    /// Large placeholder.
    Lg,
}

impl From<crate::ComponentSize> for SkeletonSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Loading placeholder reserving content shape.
#[derive(IntoElement)]
pub struct Skeleton {
    id: ElementId,
    variant: SkeletonVariant,
    size: SkeletonSize,
    width: Option<f32>,
    height: Option<f32>,
    theme: Option<SkeletonTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Skeleton {
    /// Create a text-line skeleton.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let bones = Skeleton::new("profile-loading");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: SkeletonVariant::default(),
            size: SkeletonSize::default(),
            width: None,
            height: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the placeholder shape.
    pub fn variant(mut self, variant: SkeletonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the placeholder size.
    pub fn size(mut self, size: SkeletonSize) -> Self {
        self.size = size;
        self
    }

    /// Override the placeholder width in pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the placeholder height in pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: SkeletonTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Status).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Default height in pixels for the size and variant.
    fn default_height(&self, design: &DesignSystem) -> f32 {
        match self.variant {
            SkeletonVariant::Text => match self.size {
                SkeletonSize::Sm => design.typography.small_size,
                SkeletonSize::Md => design.typography.base_size,
                SkeletonSize::Lg => design.typography.large_size,
            },
            SkeletonVariant::Rectangular => match self.size {
                SkeletonSize::Sm => design.interaction.min_touch_target,
                SkeletonSize::Md => design.interaction.min_touch_target * 2.0,
                SkeletonSize::Lg => design.interaction.min_touch_target * 3.0,
            },
            SkeletonVariant::Circular => self.default_diameter(design),
        }
    }

    /// Default width in pixels for the size and variant.
    ///
    /// Returns `None` for text lines, which stretch full width.
    fn default_width(&self, design: &DesignSystem) -> Option<f32> {
        match self.variant {
            SkeletonVariant::Text => None,
            SkeletonVariant::Rectangular => Some(match self.size {
                SkeletonSize::Sm => design.interaction.min_touch_target * 2.0,
                SkeletonSize::Md => design.interaction.min_touch_target * 3.0,
                SkeletonSize::Lg => design.interaction.min_touch_target * 4.0,
            }),
            SkeletonVariant::Circular => Some(self.default_diameter(design)),
        }
    }

    /// Default circle diameter in pixels.
    fn default_diameter(&self, design: &DesignSystem) -> f32 {
        match self.size {
            SkeletonSize::Sm => design.interaction.min_touch_target,
            SkeletonSize::Md => design.interaction.min_touch_target * 1.5,
            SkeletonSize::Lg => design.interaction.min_touch_target * 2.0,
        }
    }

    /// Build the placeholder with theme.
    pub fn build_with_theme(self, theme: &SkeletonTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the placeholder with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &SkeletonTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let height = self.height.unwrap_or_else(|| self.default_height(design));
        let width = self.width.or_else(|| self.default_width(design));

        let mut bones = div().id(self.id).flex().overflow_hidden().bg(theme.base);
        match self.variant {
            SkeletonVariant::Text => {
                bones = bones.rounded(px(design.corners.sm)).h(px(height));
                if let Some(width) = width {
                    bones = bones.w(px(width));
                } else {
                    bones = bones.w_full();
                }
            }
            SkeletonVariant::Rectangular => {
                bones = bones.rounded(px(design.corners.md)).h(px(height));
                if let Some(width) = width {
                    bones = bones.w(px(width));
                }
            }
            SkeletonVariant::Circular => {
                let diameter = width.unwrap_or(height);
                bones = bones.rounded_full().w(px(diameter)).h(px(diameter));
            }
        }

        // Static shimmer band; capture harnesses render it deterministically.
        let shimmer = div()
            .w(px(height * 2.0))
            .h_full()
            .bg(theme.shimmer)
            .opacity(0.6);
        bones = bones.child(shimmer);

        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| SharedString::from("Loading content"));
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Status));
        apply_native_accessibility(bones, native_label, &native_props)
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from("Loading content")),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Status)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| SkeletonTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Skeleton, SkeletonSize, SkeletonTheme, SkeletonVariant};
    use crate::design::neutral_design;
    use gpui::rgba;

    #[test]
    fn skeleton_defaults_to_medium_text_line() {
        let bones = Skeleton::new("loading");
        assert_eq!(bones.variant, SkeletonVariant::Text);
        assert_eq!(bones.size, SkeletonSize::Md);
        assert!(bones.width.is_none());
        assert!(bones.height.is_none());
    }

    #[test]
    fn skeleton_builders_store_shape_and_overrides() {
        let bones = Skeleton::new("loading")
            .variant(SkeletonVariant::Circular)
            .size(SkeletonSize::Lg)
            .width(64.0)
            .height(48.0);
        assert_eq!(bones.variant, SkeletonVariant::Circular);
        assert_eq!(bones.size, SkeletonSize::Lg);
        assert_eq!(bones.width, Some(64.0));
        assert_eq!(bones.height, Some(48.0));
    }

    #[test]
    fn skeleton_text_lines_stretch_full_width() {
        let design = neutral_design();
        let bones = Skeleton::new("loading");
        assert!(bones.default_width(&design).is_none());
        assert!(bones.default_height(&design) > 0.0);
    }

    #[test]
    fn skeleton_circle_uses_diameter_for_both_axes() {
        let design = neutral_design();
        let bones = Skeleton::new("loading").variant(SkeletonVariant::Circular);
        let diameter = bones.default_diameter(&design);
        assert_eq!(bones.default_width(&design), Some(diameter));
        assert_eq!(bones.default_height(&design), diameter);
    }

    #[test]
    fn skeleton_theme_defaults_match_compiled_tokens() {
        let theme = SkeletonTheme::default();
        assert_eq!(theme.base, rgba(0x2e2e2eff));
        assert_eq!(theme.shimmer, rgba(0x4a4a4aff));
    }
}
