//! Dialog/Modal component
//!
//! A modal dialog with backdrop, title, content, and footer sections.
//!
//! # Composition Patterns
//!
//! Dialogs support two composition patterns:
//!
//! ## Static content (simple)
//! ```ignore
//! Dialog::new("my-dialog")
//!     .title("Settings")
//!     .content(div().child("Dialog body"))
//!     .footer(div().child("Footer buttons"))
//! ```
//!
//! ## Dynamic content with theme access
//! ```ignore
//! Dialog::new("my-dialog")
//!     .title("Settings")
//!     .content_with(|theme| {
//!         div()
//!             .text_color(theme.title)
//!             .child("Themed content")
//!             .into_any_element()
//!     })
//! ```

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::focus::FocusTrap;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, FontWeight, KeyDownEvent, MouseButton,
    MouseDownEvent, Rems, Rgba, ScrollWheelEvent, SharedString, Window, div, px,
};
use std::rc::Rc;

fn ignore_scroll_wheel(_event: &ScrollWheelEvent, _window: &mut Window, _cx: &mut App) {}

fn ignore_mouse_down(_event: &MouseDownEvent, _window: &mut Window, _cx: &mut App) {}

/// Factory function type for creating elements with dialog theme access
pub type DialogSlotFactory = Box<dyn FnOnce(&DialogTheme) -> AnyElement>;

/// Theme colors for dialog styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct DialogTheme {
    /// Backdrop background
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
    /// Dialog background
    #[theme(default = 0x1e1e1e, from = surface)]
    pub background: Rgba,
    /// Border color
    #[theme(default = 0x007acc, from = accent)]
    pub border: Rgba,
    /// Header border
    #[theme(default = 0x3a3a3a, from = border)]
    pub header_border: Rgba,
    /// Title text color
    #[theme(default = 0xffffff, from = text_primary)]
    pub title: Rgba,
    /// Close button text
    #[theme(default = 0x888888, from = text_muted)]
    pub close: Rgba,
    /// Close button hover
    #[theme(default = 0xffffff, from = text_primary)]
    pub close_hover: Rgba,
    /// Close button hover background
    #[theme(default = 0x3a3a3a, from = surface_hover)]
    pub close_hover_bg: Rgba,
}

/// Dialog size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogSize {
    /// Small dialog (320px)
    Sm,
    /// Medium dialog (480px)
    #[default]
    Md,
    /// Large dialog (640px)
    Lg,
    /// Extra large dialog (800px)
    Xl,
    /// Full width (90%)
    Full,
}

impl DialogSize {
    fn width(&self) -> Rems {
        match self {
            DialogSize::Sm => Rems(20.0),
            DialogSize::Md => Rems(30.0),
            DialogSize::Lg => Rems(40.0),
            DialogSize::Xl => Rems(50.0),
            DialogSize::Full => Rems(60.0),
        }
    }
}

/// A modal dialog component
pub struct Dialog {
    id: ElementId,
    title: Option<SharedString>,
    size: DialogSize,
    content: Option<AnyElement>,
    content_factory: Option<DialogSlotFactory>,
    footer: Option<AnyElement>,
    footer_factory: Option<DialogSlotFactory>,
    show_close_button: bool,
    close_on_backdrop: bool,
    /// Trap Tab inside the dialog while it is focused (Radix-Dialog parity).
    trap_focus: bool,
    /// Focus handles that Tab/Shift+Tab cycle through while the trap is active.
    ///
    /// Register the same handles the dialog's interactive children use. When
    /// empty, the trap only stops Tab propagation at the dialog boundary.
    trap_targets: Vec<FocusHandle>,
    /// Close on Escape when a close handler is set.
    dismiss_on_escape: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Dialog {
    /// Create a new dialog
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            size: DialogSize::default(),
            content: None,
            content_factory: None,
            footer: None,
            footer_factory: None,
            show_close_button: true,
            close_on_backdrop: true,
            trap_focus: true,
            trap_targets: Vec::new(),
            dismiss_on_escape: true,
            focus_handle: None,
            restore_focus_to: None,
            on_close: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the dialog title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the dialog size
    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Set the dialog content
    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Alias for content (matches adabraka-ui API)
    pub fn child(self, element: impl IntoElement) -> Self {
        self.content(element)
    }

    /// Set the dialog footer
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer = Some(element.into_any_element());
        self
    }

    /// Set the dialog content with a factory function that receives the dialog theme
    ///
    /// This allows dynamic content creation with access to theme colors.
    ///
    /// # Example
    /// ```ignore
    /// Dialog::new("dialog")
    ///     .content_with(|theme| {
    ///         div()
    ///             .text_color(theme.title)
    ///             .child("Themed content")
    ///             .into_any_element()
    ///     })
    /// ```
    pub fn content_with(
        mut self,
        factory: impl FnOnce(&DialogTheme) -> AnyElement + 'static,
    ) -> Self {
        self.content_factory = Some(Box::new(factory));
        self
    }

    /// Set the dialog footer with a factory function that receives the dialog theme
    ///
    /// # Example
    /// ```ignore
    /// Dialog::new("dialog")
    ///     .footer_with(|theme| {
    ///         div()
    ///             .border_t_1()
    ///             .border_color(theme.header_border)
    ///             .child("Footer with theme")
    ///             .into_any_element()
    ///     })
    /// ```
    pub fn footer_with(
        mut self,
        factory: impl FnOnce(&DialogTheme) -> AnyElement + 'static,
    ) -> Self {
        self.footer_factory = Some(Box::new(factory));
        self
    }

    /// Show or hide the close button
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    /// Close dialog when clicking backdrop
    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    /// Trap Tab key presses inside the dialog while it is focused.
    ///
    /// Enabled by default. The dialog stops Tab propagation at its boundary
    /// so keyboard focus cannot escape to the page behind the modal; inner
    /// fields still receive the key first. Requires [`Self::focus_handle`]
    /// for key handling. Disabling restores plain bubbling.
    pub fn trap_focus(mut self, trap: bool) -> Self {
        self.trap_focus = trap;
        self
    }

    /// Cycle Tab/Shift+Tab through these focus handles while the trap is active.
    ///
    /// Pass the handles of the dialog's interactive children. Tab wraps from
    /// the last target to the first (Shift+Tab reverses) and never propagates
    /// past the dialog. Requires [`Self::focus_handle`] for key handling.
    pub fn trap_targets(mut self, handles: impl IntoIterator<Item = FocusHandle>) -> Self {
        self.trap_targets = handles.into_iter().collect();
        self
    }

    /// Add one focus handle to the Tab trap cycle.
    pub fn trap_target(mut self, handle: FocusHandle) -> Self {
        self.trap_targets.push(handle);
        self
    }

    /// Close the dialog on Escape when a close handler is set.
    ///
    /// Enabled by default. Requires [`Self::focus_handle`] for key handling.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    /// Set the focus handle used for dialog-level keyboard dismissal.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the focus handle to restore before running the close callback.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus_to = Some(handle);
        self
    }

    /// Set the close handler
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label (overrides the dialog's title)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Dialog)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the dialog into elements with theme
    pub fn build_with_theme(self, theme: &DialogTheme) -> Div {
        let width = self.size.width();
        let close_on_backdrop = self.close_on_backdrop;
        let focus_handle = self.focus_handle.clone();
        let restore_focus_to = self.restore_focus_to.clone();
        // Clone ID for use in child elements (self.id is moved to dialog container)
        let close_btn_id = self.id.clone();
        let content_id = self.id.clone();

        // Convert Box to Rc for shared ownership between backdrop and close button
        let on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|f| Rc::from(f));

        // Backdrop
        let mut backdrop = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            // Capture scroll events to prevent propagation to underlying view
            .on_scroll_wheel(ignore_scroll_wheel);

        // Handle backdrop click
        if close_on_backdrop && let Some(handler) = on_close.clone() {
            let restore_focus_to = restore_focus_to.clone();
            backdrop = backdrop.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                if let Some(ref handle) = restore_focus_to {
                    window.focus(handle, cx);
                }
                handler(window, cx);
            });
        }

        // Dialog container
        let mut dialog = div()
            .id(self.id)
            .w(width)
            .max_h(Rems(45.0))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            // Stop propagation so clicking dialog doesn't close it
            .on_mouse_down(MouseButton::Left, ignore_mouse_down);

        if let Some(handle) = focus_handle.clone() {
            dialog = dialog.track_focus(&handle).focusable();
            let trap_focus = self.trap_focus;
            let trap_targets = self.trap_targets;
            let dismiss_on_escape = self.dismiss_on_escape;
            if on_close.is_some() || trap_focus {
                let handler = on_close.clone();
                let restore_focus_to = restore_focus_to.clone();
                dialog = dialog.on_key_down(
                    move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                        let key = event.keystroke.key.as_str();
                        // Focus trap: keep Tab inside the modal. Inner fields
                        // run their own handlers first (bubbling); cycling (or
                        // stopping propagation when no targets are registered)
                        // here only prevents focus escaping the dialog.
                        if trap_focus && key == "tab" {
                            if !trap_targets.is_empty() {
                                let current = trap_targets
                                    .iter()
                                    .position(|target| target.is_focused(window));
                                if let Some(index) = FocusTrap::cycle_index(
                                    trap_targets.len(),
                                    current,
                                    event.keystroke.modifiers.shift,
                                ) {
                                    window.focus(&trap_targets[index], cx);
                                }
                                cx.stop_propagation();
                                return;
                            }
                            if handle.is_focused(window) {
                                cx.stop_propagation();
                                return;
                            }
                        }
                        if !dismiss_on_escape {
                            return;
                        }
                        if crate::interaction::overlay_key_action(key, handle.is_focused(window))
                            != Some(crate::interaction::OverlayKeyAction::Dismiss)
                        {
                            return;
                        }
                        let Some(handler) = handler.as_ref() else {
                            return;
                        };

                        cx.stop_propagation();
                        if let Some(ref handle) = restore_focus_to {
                            window.focus(handle, cx);
                        }
                        handler(window, cx);
                    },
                );
            }
        }

        // Header with title and close button
        if self.title.is_some() || self.show_close_button {
            let mut header = div()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme.header_border);

            if let Some(title) = self.title {
                header = header.child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.title)
                        .child(title),
                );
            } else {
                header = header.child(div()); // Spacer
            }

            if self.show_close_button
                && let Some(handler) = on_close.clone()
            {
                let restore_focus_to = restore_focus_to.clone();
                let close_color = theme.close;
                let close_hover = theme.close_hover;
                let close_hover_bg = theme.close_hover_bg;
                header = header.child(
                    div()
                        .id((close_btn_id, "close"))
                        .px_2()
                        .py_1()
                        .rounded(px(3.0))
                        .cursor_pointer()
                        .text_color(close_color)
                        .hover(move |s| s.bg(close_hover_bg).text_color(close_hover))
                        .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                            if let Some(ref handle) = restore_focus_to {
                                window.focus(handle, cx);
                            }
                            handler(window, cx);
                        })
                        .child("×"),
                );
            }

            dialog = dialog.child(header);
        }

        // Content - factory takes precedence over static element
        let content_element = self.content_factory.map(|f| f(theme)).or(self.content);
        if let Some(content) = content_element {
            dialog = dialog.child(
                div()
                    .id((content_id, "content"))
                    .flex_1()
                    .overflow_y_scroll()
                    .px_4()
                    .py_4()
                    .child(content),
            );
        }

        // Footer - factory takes precedence over static element
        let footer_element = self.footer_factory.map(|f| f(theme)).or(self.footer);
        if let Some(footer) = footer_element {
            dialog = dialog.child(
                div()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.header_border)
                    .child(footer),
            );
        }

        backdrop.child(dialog)
    }
}

impl RenderOnce for Dialog {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.title.clone())
            .unwrap_or_default();
        let modal = self.trap_focus;
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Dialog))
                .maybe_state(modal, crate::accessibility::AriaState::Modal),
        });

        let global_theme = cx.theme();
        let dialog_theme = DialogTheme::from(global_theme);
        self.build_with_theme(&dialog_theme)
    }
}

impl IntoElement for Dialog {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Dialog, DialogSize};

    #[test]
    fn dialog_builder_records_keyboard_focus_contract() {
        let dialog = Dialog::new("settings")
            .title("Settings")
            .size(DialogSize::Lg)
            .close_on_backdrop(false)
            .show_close_button(false)
            .on_close(|_, _| {});

        assert_eq!(dialog.title.as_deref(), Some("Settings"));
        assert_eq!(dialog.size, DialogSize::Lg);
        assert!(!dialog.close_on_backdrop);
        assert!(!dialog.show_close_button);
        assert!(dialog.on_close.is_some());
    }

    #[test]
    fn dialog_traps_focus_and_dismisses_on_escape_by_default() {
        let dialog = Dialog::new("modal").on_close(|_, _| {});
        assert!(dialog.trap_focus);
        assert!(dialog.dismiss_on_escape);

        let relaxed = Dialog::new("plain")
            .trap_focus(false)
            .dismiss_on_escape(false);
        assert!(!relaxed.trap_focus);
        assert!(!relaxed.dismiss_on_escape);
    }

    #[test]
    fn dialog_defaults_to_empty_trap_targets() {
        let dialog = Dialog::new("modal").on_close(|_, _| {});
        assert!(dialog.trap_targets.is_empty());
    }
}
