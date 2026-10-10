//! Badge component
//!
//! Small status indicators and labels.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, Pixels, Rgba, SharedString, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Badge variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    /// Default gray
    #[default]
    Default,
    /// Primary blue
    Primary,
    /// Success green
    Success,
    /// Warning yellow
    Warning,
    /// Error red
    Error,
    /// Info cyan
    Info,
}

impl BadgeVariant {
    fn colors(&self, theme: &Theme) -> (Rgba, Rgba) {
        // Returns (background, text_color) from theme
        match self {
            BadgeVariant::Default => (theme.surface, theme.text_secondary),
            BadgeVariant::Primary => (theme.badge_primary_bg, theme.badge_primary_text),
            BadgeVariant::Success => (theme.badge_success_bg, theme.badge_success_text),
            BadgeVariant::Warning => (theme.badge_warning_bg, theme.badge_warning_text),
            BadgeVariant::Error => (theme.badge_error_bg, theme.badge_error_text),
            BadgeVariant::Info => (theme.badge_info_bg, theme.badge_info_text),
        }
    }
}

/// Badge size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl From<crate::ComponentSize> for BadgeSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A badge component
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    variant: BadgeVariant,
    size: BadgeSize,
    rounded: bool,
    icon: Option<SharedString>,
    design: Option<Arc<DesignSystem>>,
}

impl Badge {
    /// Create a new badge
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            variant: BadgeVariant::default(),
            size: BadgeSize::default(),
            rounded: false,
            icon: None,
            design: None,
        }
    }

    /// Set the design system (falls back to the app-global design when unset)
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set variant
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    /// Make fully rounded (pill shape)
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Add icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Build into element with theme (uses the neutral design for geometry)
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build into element with explicit theme and design
    pub fn build_with_theme_and_design(self, theme: &Theme, design: &DesignSystem) -> Div {
        let (bg, text_color) = self.variant.colors(theme);

        let (px_val, py_val) = match self.size {
            BadgeSize::Sm => (
                px(design.spacing.grid_unit * 1.5),
                px(design.spacing.grid_unit * 0.5),
            ),
            BadgeSize::Md => (
                px(design.spacing.control_gap),
                px(design.spacing.grid_unit * 0.75),
            ),
            BadgeSize::Lg => (
                px(design.spacing.control_padding_x),
                px(design.spacing.grid_unit),
            ),
        };

        let mut badge = div()
            .flex()
            .items_center()
            .gap(px(design.spacing.grid_unit))
            .px(px_val)
            .py(py_val)
            .bg(bg)
            .text_color(text_color);

        // Apply text size
        badge = match self.size {
            BadgeSize::Sm | BadgeSize::Md => badge.text_size(px(design.typography.small_size)),
            BadgeSize::Lg => badge.text_size(px(design.typography.base_size)),
        };

        // Apply rounding
        if self.rounded {
            badge = badge.rounded_full();
        } else {
            badge = badge.rounded(px(design.corners.sm * 0.75));
        }

        // Icon
        if let Some(icon) = self.icon {
            badge = badge.child(div().child(icon));
        }

        // Label
        badge = badge.child(self.label);

        badge
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let design = crate::design::resolve_design(self.design.clone(), cx);
        let theme = cx.theme();
        self.build_with_theme_and_design(&theme, &design)
    }
}

/// A dot indicator (no text)
#[derive(IntoElement)]
pub struct BadgeDot {
    variant: BadgeVariant,
    size: Pixels,
}

impl BadgeDot {
    /// Create a new badge dot
    pub fn new() -> Self {
        Self {
            variant: BadgeVariant::default(),
            size: px(8.0),
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size in pixels
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let (bg, _) = self.variant.colors(theme);
        div().w(self.size).h(self.size).rounded_full().bg(bg)
    }
}

impl Default for BadgeDot {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for BadgeDot {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}
