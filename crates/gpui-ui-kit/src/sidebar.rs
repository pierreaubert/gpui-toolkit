//! Sidebar component
//!
//! A collapsible side panel with optional animation support.
//!
//! # Usage
//!
//! ```ignore
//! Sidebar::new("nav-sidebar")
//!     .width(px(260.0))
//!     .collapsed(is_collapsed)
//!     .side(SidebarSide::Left)
//!     .content(nav_element)
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, Pixels, Rgba, ScrollHandle, ScrollWheelEvent, Stateful,
    Window, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Which side the sidebar is on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarSide {
    /// Left side (default)
    #[default]
    Left,
    /// Right side
    Right,
}

/// Factory function for creating sidebar content with theme access
pub type SidebarSlotFactory = Box<dyn FnOnce(&SidebarTheme) -> AnyElement>;
type SidebarScrollListener = Box<dyn Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static>;

/// Theme colors for sidebar styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SidebarTheme {
    /// Sidebar background
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Sidebar border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
}

/// A collapsible sidebar component
pub struct Sidebar {
    id: ElementId,
    side: SidebarSide,
    width: Pixels,
    collapsed: bool,
    content: Option<AnyElement>,
    content_factory: Option<SidebarSlotFactory>,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    show_border: bool,
    design: Option<Arc<DesignSystem>>,
    scroll_handle: Option<ScrollHandle>,
    scroll_listener: Option<SidebarScrollListener>,
}

impl Sidebar {
    /// Create a new sidebar
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            side: SidebarSide::default(),
            width: px(260.0),
            collapsed: false,
            content: None,
            content_factory: None,
            header: None,
            footer: None,
            show_border: true,
            design: None,
            scroll_handle: None,
            scroll_listener: None,
        }
    }

    /// Set which side the sidebar is on
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    /// Set expanded width
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Set collapsed state
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Set the sidebar content
    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Set content with theme access
    pub fn content_with(
        mut self,
        factory: impl FnOnce(&SidebarTheme) -> AnyElement + 'static,
    ) -> Self {
        self.content_factory = Some(Box::new(factory));
        self
    }

    /// Set a header element (pinned to top)
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header = Some(element.into_any_element());
        self
    }

    /// Set a footer element (pinned to bottom)
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer = Some(element.into_any_element());
        self
    }

    /// Show or hide the border on the inner edge
    pub fn show_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// Override the design system used for sidebar sizing defaults.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Track the sidebar content scroll state.
    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(handle.clone());
        self
    }

    /// Listen to scroll wheel events on the sidebar content.
    pub fn on_scroll_wheel(
        mut self,
        listener: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.scroll_listener = Some(Box::new(listener));
        self
    }

    /// Build the sidebar with theme
    pub fn build_with_theme(self, theme: &SidebarTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the sidebar with theme and design-system sizing tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &SidebarTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        if self.collapsed {
            return div().id(self.id).w(px(0.0)).overflow_hidden();
        }

        let content_id = (self.id.clone(), "sidebar-content");
        let mut sidebar = div()
            .id(self.id)
            .w(self.width)
            .min_w(px(design.interaction.min_touch_target))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.background)
            .overflow_hidden();

        // Border on the inner edge
        if self.show_border {
            sidebar = match self.side {
                SidebarSide::Left => sidebar.border_r_1().border_color(theme.border),
                SidebarSide::Right => sidebar.border_l_1().border_color(theme.border),
            };
        }

        // Header (pinned to top)
        if let Some(header) = self.header {
            sidebar = sidebar.child(div().flex_shrink_0().child(header));
        }

        // Content (scrollable, fills remaining space)
        let content_element = self.content_factory.map(|f| f(theme)).or(self.content);
        if let Some(content) = content_element {
            let mut content_container = div().id(content_id).flex_1().min_h_0().overflow_y_scroll();
            if let Some(scroll_handle) = self.scroll_handle.as_ref() {
                content_container = content_container.track_scroll(scroll_handle);
            }
            if let Some(scroll_listener) = self.scroll_listener {
                content_container = content_container.on_scroll_wheel(scroll_listener);
            }

            sidebar = sidebar.child(
                content_container.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .flex_shrink_0()
                        .child(content),
                ),
            );
        }

        // Footer (pinned to bottom)
        if let Some(footer) = self.footer {
            sidebar = sidebar.child(div().flex_shrink_0().child(footer));
        }

        sidebar
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = SidebarTheme::from(global_theme);
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

impl IntoElement for Sidebar {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
