//! ContextMenu component
//!
//! A positioned context menu (right-click menu) that wraps the Menu component
//! with positioning logic and backdrop dismiss behavior.
//!
//! # Usage
//!
//! ```ignore
//! // Show context menu at mouse position
//! ContextMenu::new("ctx-menu", items)
//!     .position(mouse_position)
//!     .on_select(|id, window, cx| { /* handle selection */ })
//!     .on_close(|window, cx| { /* dismiss */ })
//! ```

use crate::ComponentTheme;
use crate::menu::{MenuItem, MenuTheme};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FocusHandle, MouseButton, Pixels, Point, Rgba, SharedString, Window, div,
    point, px,
};
use std::rc::Rc;

/// Theme colors for context menu styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ContextMenuTheme {
    /// Backdrop overlay color (usually transparent or very subtle)
    #[theme(default = 0x00000001, from_expr = "gpui::rgba(0x00000001)")]
    pub backdrop: Rgba,
    /// Menu background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background: Rgba,
    /// Menu border
    #[theme(default = 0x444444ff, from = border)]
    pub border: Rgba,
    /// Separator color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub separator: Rgba,
    /// Normal item text
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Hovered item text
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_hover: Rgba,
    /// Disabled item text
    #[theme(default = 0x666666ff, from = text_muted)]
    pub text_disabled: Rgba,
    /// Shortcut text color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub text_shortcut: Rgba,
    /// Item hover background
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub hover_bg: Rgba,
    /// Danger item hover background
    #[theme(default = 0xdc2626ff, from = error)]
    pub danger_hover_bg: Rgba,
}

impl ContextMenuTheme {
    /// Convert to a MenuTheme for rendering the inner Menu
    pub fn to_menu_theme(&self) -> MenuTheme {
        MenuTheme {
            background: self.background,
            border: self.border,
            separator: self.separator,
            text: self.text,
            text_hover: self.text_hover,
            text_disabled: self.text_disabled,
            text_shortcut: self.text_shortcut,
            hover_bg: self.hover_bg,
            danger_hover_bg: self.danger_hover_bg,
        }
    }
}

/// A context menu component that renders a Menu at a specific position
/// with a transparent backdrop for click-outside dismiss.
pub struct ContextMenu {
    id: ElementId,
    items: Vec<MenuItem>,
    position: Point<Pixels>,
    min_width: Pixels,
    focused_index: Option<usize>,
    focus_handle: Option<FocusHandle>,
    on_select: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_focus_change: Option<Box<dyn Fn(Option<usize>, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
}

impl ContextMenu {
    /// Create a new context menu
    pub fn new(id: impl Into<ElementId>, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.into(),
            items,
            position: point(px(0.0), px(0.0)),
            min_width: px(180.0),
            focused_index: None,
            focus_handle: None,
            on_select: None,
            on_close: None,
            on_focus_change: None,
            aria_label: Some("Context menu".into()),
        }
    }

    /// Set the position where the menu appears (typically mouse position)
    pub fn position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }

    /// Set minimum width
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = width;
        self
    }

    /// Set the currently focused item index
    pub fn focused_index(mut self, index: usize) -> Self {
        self.focused_index = Some(index);
        self
    }

    /// Set the focus handle for keyboard navigation
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the selection handler
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Set the close handler (backdrop click or Escape)
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Set the focus change handler (arrow keys)
    pub fn on_focus_change(
        mut self,
        handler: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_focus_change = Some(Box::new(handler));
        self
    }

    /// Set the accessible name of the menu region.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Build the context menu with theme
    pub fn build_with_theme(self, theme: &ContextMenuTheme) -> Div {
        let on_close_rc: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|f| Rc::from(f));

        // Transparent full-screen backdrop to catch outside clicks
        let mut backdrop = div()
            .absolute()
            .inset_0()
            .bg(theme.backdrop)
            .on_scroll_wheel(|_event, _window, _cx| {});

        if let Some(handler) = on_close_rc.clone() {
            backdrop = backdrop.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                handler(window, cx);
            });
        }

        // Build the menu using the existing Menu component
        let menu_theme = theme.to_menu_theme();
        let mut menu = crate::menu::Menu::new(self.id, self.items).min_width(self.min_width);
        if let Some(label) = self.aria_label {
            menu = menu.aria_label(label);
        }

        if let Some(idx) = self.focused_index {
            menu = menu.focused_index(idx);
        }
        if let Some(handle) = self.focus_handle {
            menu = menu.focus_handle(handle);
        }
        if let Some(handler) = self.on_select {
            menu = menu.on_select(handler);
        }
        if let Some(handler) = on_close_rc {
            let handler_clone = handler.clone();
            menu = menu.on_close(move |window, cx| handler_clone(window, cx));
        }
        if let Some(handler) = self.on_focus_change {
            menu = menu.on_focus_change(handler);
        }

        // Position the menu at the specified coordinates
        let pos = self.position;
        backdrop.child(
            div()
                .absolute()
                .left(pos.x)
                .top(pos.y)
                // Stop propagation so clicking menu doesn't close it
                .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {})
                .child(menu.build_with_theme(&menu_theme)),
        )
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = ContextMenuTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for ContextMenu {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
