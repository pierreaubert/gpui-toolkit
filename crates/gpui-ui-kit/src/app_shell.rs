//! `AppShell` component
//!
//! A generic application frame with header, sidebar, content, and footer
//! slots. The shell only arranges regions; it never composes product
//! navigation, sidebars, or status bars itself. Callers fill each slot
//! with their own elements, and empty slots render nothing.
//!
//! # Usage
//!
//! ```ignore
//! AppShell::new("app")
//!     .header(TopNav::new("nav"))
//!     .sidebar(Sidebar::new("side"))
//!     .content(div().child("Main view"))
//!     .footer(StatusBar::new("status"))
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, Pixels, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Edge the shell sidebar hugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppShellSidebarSide {
    /// Sidebar on the left (default).
    #[default]
    Left,
    /// Sidebar on the right.
    Right,
}

/// Theme colors for shell regions.
#[derive(Debug, Clone, ComponentTheme)]
pub struct AppShellTheme {
    /// Shell background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Dividers between shell regions.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
}

/// Generic application frame with named slots.
///
/// The shell is a layout primitive: it positions a header, an optional
/// sidebar, main content, and a footer. Product components such as
/// `TopNav`, `Sidebar`, or `StatusBar` are composed by the caller.
#[derive(IntoElement)]
pub struct AppShell {
    id: ElementId,
    header: Option<AnyElement>,
    sidebar: Option<AnyElement>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    sidebar_side: AppShellSidebarSide,
    sidebar_width: Option<Pixels>,
    theme: Option<AppShellTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl AppShell {
    /// Create a new empty shell.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            header: None,
            sidebar: None,
            content: None,
            footer: None,
            sidebar_side: AppShellSidebarSide::default(),
            sidebar_width: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the header slot.
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header = Some(element.into_any_element());
        self
    }

    /// Set the sidebar slot.
    pub fn sidebar(mut self, element: impl IntoElement) -> Self {
        self.sidebar = Some(element.into_any_element());
        self
    }

    /// Set the main content slot.
    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Set the footer slot.
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer = Some(element.into_any_element());
        self
    }

    /// Set which edge the sidebar hugs.
    pub fn sidebar_side(mut self, side: AppShellSidebarSide) -> Self {
        self.sidebar_side = side;
        self
    }

    /// Set an explicit sidebar width.
    pub fn sidebar_width(mut self, width: Pixels) -> Self {
        self.sidebar_width = Some(width);
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: AppShellTheme) -> Self {
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

    /// Override the default ARIA role (Region).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Default sidebar width from touch target.
    fn default_sidebar_width(design: &DesignSystem) -> Pixels {
        px(design.interaction.min_touch_target * 6.0)
    }

    /// Build the shell with theme.
    pub fn build_with_theme(self, theme: &AppShellTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the shell with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &AppShellTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| SharedString::from("Application shell"));
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Region));

        let sidebar_width = self
            .sidebar_width
            .unwrap_or(Self::default_sidebar_width(design));
        let border = theme.border;

        let sidebar_element = self.sidebar.map(|sidebar| {
            let mut panel = div()
                .w(sidebar_width)
                .flex_shrink_0()
                .h_full()
                .overflow_hidden()
                .child(sidebar);
            panel = match self.sidebar_side {
                AppShellSidebarSide::Left => panel.border_r_1().border_color(border),
                AppShellSidebarSide::Right => panel.border_l_1().border_color(border),
            };
            panel
        });
        let content_element = self.content.map(|content| {
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .overflow_hidden()
                .child(content)
        });

        let mut middle = div().flex().flex_1().min_h_0().min_w_0();
        match self.sidebar_side {
            AppShellSidebarSide::Left => {
                middle = middle.children(sidebar_element).children(content_element);
            }
            AppShellSidebarSide::Right => {
                middle = middle.children(content_element).children(sidebar_element);
            }
        }

        let mut shell = div()
            .id(self.id)
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background);

        if let Some(header) = self.header {
            shell = shell.child(
                div()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(border)
                    .child(header),
            );
        }
        shell = shell.child(middle);
        if let Some(footer) = self.footer {
            shell = shell.child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(border)
                    .child(footer),
            );
        }

        apply_native_accessibility(shell, native_label, &native_props)
    }
}

impl RenderOnce for AppShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from("Application shell")),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Region)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| AppShellTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppShell, AppShellSidebarSide, AppShellTheme};
    use gpui::rgba;

    #[test]
    fn shell_defaults_to_empty_left_sidebar_frame() {
        let shell = AppShell::new("app");
        assert_eq!(shell.sidebar_side, AppShellSidebarSide::Left);
        assert!(shell.sidebar_width.is_none());
        assert!(shell.header.is_none());
        assert!(shell.sidebar.is_none());
        assert!(shell.content.is_none());
        assert!(shell.footer.is_none());
        assert!(shell.theme.is_none());
        assert!(shell.aria_label.is_none());
        assert!(shell.aria_role.is_none());
    }

    #[test]
    fn shell_builders_store_slots_and_side() {
        let shell = AppShell::new("app")
            .header(gpui::div())
            .sidebar(gpui::div())
            .content(gpui::div())
            .footer(gpui::div())
            .sidebar_side(AppShellSidebarSide::Right)
            .sidebar_width(gpui::px(300.0))
            .aria_label("Demo shell");
        assert!(shell.header.is_some());
        assert!(shell.sidebar.is_some());
        assert!(shell.content.is_some());
        assert!(shell.footer.is_some());
        assert_eq!(shell.sidebar_side, AppShellSidebarSide::Right);
        assert_eq!(shell.sidebar_width, Some(gpui::px(300.0)));
        assert_eq!(
            shell.aria_label,
            Some(gpui::SharedString::from("Demo shell"))
        );
    }

    #[test]
    fn shell_theme_defaults_match_compiled_tokens() {
        let theme = AppShellTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
    }

    #[test]
    fn shell_default_sidebar_width_scales_with_touch_target() {
        let design = crate::design::neutral_design();
        let width = AppShell::default_sidebar_width(&design);
        assert_eq!(width, gpui::px(design.interaction.min_touch_target * 6.0));
        assert!(width > gpui::px(0.0));
    }
}
