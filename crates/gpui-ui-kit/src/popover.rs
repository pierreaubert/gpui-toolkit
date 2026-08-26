//! Popover component
//!
//! A floating panel anchored to a trigger element with click-outside dismiss.
//!
//! # Usage
//!
//! ```ignore
//! Popover::new("device-picker")
//!     .placement(PopoverPlacement::Bottom)
//!     .content(device_list_element)
//!     .on_close(|window, cx| { /* dismiss */ })
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, KeyDownEvent, MouseButton, Pixels, Rgba, Window,
    div,
};
use std::rc::Rc;

/// Popover placement relative to the anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverPlacement {
    /// Above the anchor
    Top,
    /// Below the anchor (default)
    #[default]
    Bottom,
    /// Left of the anchor
    Left,
    /// Right of the anchor
    Right,
    /// Top-left aligned
    TopStart,
    /// Top-right aligned
    TopEnd,
    /// Bottom-left aligned
    BottomStart,
    /// Bottom-right aligned
    BottomEnd,
}

/// Factory function for creating popover content with theme access
pub type PopoverSlotFactory = Box<dyn FnOnce(&PopoverTheme) -> AnyElement>;

/// Theme colors for popover styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct PopoverTheme {
    /// Popover background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background: Rgba,
    /// Popover border
    #[theme(default = 0x444444ff, from = border)]
    pub border: Rgba,
    /// Backdrop color (transparent by default for click-outside detection)
    #[theme(default = 0x00000001, from_expr = "gpui::rgba(0x00000001)")]
    pub backdrop: Rgba,
}

/// A popover component that floats relative to its parent container
pub struct Popover {
    id: ElementId,
    placement: PopoverPlacement,
    content: Option<AnyElement>,
    content_factory: Option<PopoverSlotFactory>,
    width: Option<Pixels>,
    show_backdrop: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Popover {
    /// Create a new popover
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            placement: PopoverPlacement::default(),
            content: None,
            content_factory: None,
            width: None,
            show_backdrop: true,
            focus_handle: None,
            restore_focus_to: None,
            on_close: None,
        }
    }

    /// Set placement
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set static content
    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Set content via factory with theme access
    pub fn content_with(
        mut self,
        factory: impl FnOnce(&PopoverTheme) -> AnyElement + 'static,
    ) -> Self {
        self.content_factory = Some(Box::new(factory));
        self
    }

    /// Set fixed width
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Whether to show a backdrop for click-outside dismiss (default: true)
    pub fn show_backdrop(mut self, show: bool) -> Self {
        self.show_backdrop = show;
        self
    }

    /// Set the focus handle used for popover-level keyboard dismissal.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the focus handle to restore before running the close callback.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus_to = Some(handle);
        self
    }

    /// Set close handler
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Build the popover with theme.
    ///
    /// Returns a relatively-positioned container. The caller should place this
    /// as a child of the anchor element (which must also be `relative`).
    pub fn build_with_theme(self, theme: &PopoverTheme) -> Div {
        let focus_handle = self.focus_handle.clone();
        let restore_focus_to = self.restore_focus_to.clone();
        let on_close_rc: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|f| Rc::from(f));

        let content_element = self.content_factory.map(|f| f(theme)).or(self.content);

        // Build the floating panel
        let mut panel = div()
            .id(self.id)
            .absolute()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .overflow_hidden()
            // Prevent click-through to the dismissing backdrop.
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            });

        if let Some(w) = self.width {
            panel = panel.w(w);
        }

        if let Some(ref handle) = focus_handle {
            panel = panel.track_focus(handle).focusable();
        }

        // Position based on placement
        panel = match self.placement {
            PopoverPlacement::Top => panel.bottom_full().left_0().mb_1(),
            PopoverPlacement::Bottom => panel.top_full().left_0().mt_1(),
            PopoverPlacement::Left => panel.right_full().top_0().mr_1(),
            PopoverPlacement::Right => panel.left_full().top_0().ml_1(),
            PopoverPlacement::TopStart => panel.bottom_full().left_0().mb_1(),
            PopoverPlacement::TopEnd => panel.bottom_full().right_0().mb_1(),
            PopoverPlacement::BottomStart => panel.top_full().left_0().mt_1(),
            PopoverPlacement::BottomEnd => panel.top_full().right_0().mt_1(),
        };

        if let Some(content) = content_element {
            panel = panel.child(content);
        }

        if let Some(handle) = focus_handle.clone()
            && let Some(handler) = on_close_rc.clone()
        {
            let restore_focus_to = restore_focus_to.clone();
            panel = panel.on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if crate::interaction::overlay_key_action(
                        event.keystroke.key.as_str(),
                        handle.is_focused(window),
                    ) != Some(crate::interaction::OverlayKeyAction::Dismiss)
                    {
                        return;
                    }

                    cx.stop_propagation();
                    if let Some(ref handle) = restore_focus_to {
                        window.focus(handle, cx);
                    }
                    handler(window, cx);
                },
            );
        }

        if self.show_backdrop {
            // Full-screen transparent backdrop behind popover
            let mut backdrop = div()
                .absolute()
                .inset_0()
                .bg(theme.backdrop)
                .on_scroll_wheel(|_event, _window, _cx| {});

            if let Some(handler) = on_close_rc.clone() {
                let restore_focus_to = restore_focus_to.clone();
                backdrop = backdrop.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    if let Some(ref handle) = restore_focus_to {
                        window.focus(handle, cx);
                    }
                    handler(window, cx);
                });
            }

            // Use z-ordering: backdrop first (behind), then panel on top
            div().relative().child(backdrop).child(panel)
        } else {
            div().relative().child(panel)
        }
    }
}

impl RenderOnce for Popover {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = PopoverTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Popover {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Popover, PopoverPlacement};
    use gpui::px;

    #[test]
    fn popover_builder_records_keyboard_dismiss_contract() {
        let popover = Popover::new("filters")
            .placement(PopoverPlacement::BottomEnd)
            .width(px(240.0))
            .show_backdrop(false)
            .on_close(|_, _| {});

        assert_eq!(popover.placement, PopoverPlacement::BottomEnd);
        assert_eq!(popover.width, Some(px(240.0)));
        assert!(!popover.show_backdrop);
        assert!(popover.on_close.is_some());
    }
}
