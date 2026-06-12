//! ConfirmDialog component
//!
//! A specialized dialog for confirmation prompts with confirm/cancel actions.
//!
//! # Usage
//!
//! ```ignore
//! ConfirmDialog::new("delete-confirm")
//!     .title("Delete Album")
//!     .message("Are you sure you want to delete this album? This action cannot be undone.")
//!     .variant(ConfirmDialogVariant::Destructive)
//!     .confirm_label("Delete")
//!     .on_confirm(|window, cx| { /* perform delete */ })
//!     .on_cancel(|window, cx| { /* dismiss */ })
//! ```

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::button::{Button, ButtonVariant};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, FontWeight, MouseButton, Rems, Rgba, SharedString, Window, div};
use std::rc::Rc;

/// Confirm dialog variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmDialogVariant {
    /// Standard confirmation (blue accent)
    #[default]
    Default,
    /// Destructive action (red accent)
    Destructive,
    /// Warning (yellow accent)
    Warning,
}

/// Theme colors for confirm dialog
#[derive(Debug, Clone, ComponentTheme)]
pub struct ConfirmDialogTheme {
    /// Backdrop background
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
    /// Dialog background
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Dialog border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Title text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub title: Rgba,
    /// Message text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub message: Rgba,
    /// Destructive accent color
    #[theme(default = 0xdc2626ff, from = error)]
    pub destructive: Rgba,
    /// Warning accent color
    #[theme(default = 0xf59e0bff, from = warning)]
    pub warning: Rgba,
}

/// A confirmation dialog component
pub struct ConfirmDialog {
    id: ElementId,
    title: Option<SharedString>,
    message: SharedString,
    variant: ConfirmDialogVariant,
    confirm_label: SharedString,
    cancel_label: SharedString,
    on_confirm: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_cancel: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl ConfirmDialog {
    /// Create a new confirm dialog
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            message: "Are you sure?".into(),
            variant: ConfirmDialogVariant::default(),
            confirm_label: "Confirm".into(),
            cancel_label: "Cancel".into(),
            on_confirm: None,
            on_cancel: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the message
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = message.into();
        self
    }

    /// Set the variant
    pub fn variant(mut self, variant: ConfirmDialogVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the confirm button label
    pub fn confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Set the cancel button label
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Set the confirm handler
    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Box::new(handler));
        self
    }

    /// Set the cancel handler
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label (overrides the dialog's title)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Alertdialog)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the confirm dialog with theme
    pub fn build_with_theme(self, theme: &ConfirmDialogTheme) -> Div {
        let on_confirm_rc: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_confirm.map(|f| Rc::from(f));
        let on_cancel_rc: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_cancel.map(|f| Rc::from(f));

        // Backdrop
        let mut backdrop = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .on_scroll_wheel(|_event, _window, _cx| {});

        // Click backdrop to cancel
        if let Some(ref handler) = on_cancel_rc {
            let handler = handler.clone();
            backdrop = backdrop.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                handler(window, cx);
            });
        }

        // Determine border accent color based on variant
        let accent_border = match self.variant {
            ConfirmDialogVariant::Default => theme.border,
            ConfirmDialogVariant::Destructive => theme.destructive,
            ConfirmDialogVariant::Warning => theme.warning,
        };

        // Dialog container
        let mut dialog = div()
            .id(self.id.clone())
            .w(Rems(26.0))
            .bg(theme.background)
            .border_1()
            .border_color(accent_border)
            .rounded_lg()
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {});

        // Title
        if let Some(title) = self.title {
            dialog = dialog.child(
                div()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .text_base()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.title)
                    .child(title),
            );
        }

        // Message
        dialog = dialog.child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .text_color(theme.message)
                .child(self.message),
        );

        // Footer with buttons
        let confirm_variant = match self.variant {
            ConfirmDialogVariant::Default => ButtonVariant::Primary,
            ConfirmDialogVariant::Destructive => ButtonVariant::Destructive,
            ConfirmDialogVariant::Warning => ButtonVariant::Primary,
        };

        let mut cancel_btn = Button::new((self.id.clone(), "cancel"), self.cancel_label)
            .variant(ButtonVariant::Ghost);

        if let Some(ref handler) = on_cancel_rc {
            let handler = handler.clone();
            cancel_btn = cancel_btn.on_click(move |window, cx| {
                handler(window, cx);
            });
        }

        let mut confirm_btn =
            Button::new((self.id.clone(), "confirm"), self.confirm_label).variant(confirm_variant);

        if let Some(ref handler) = on_confirm_rc {
            let handler = handler.clone();
            confirm_btn = confirm_btn.on_click(move |window, cx| {
                handler(window, cx);
            });
        }

        dialog = dialog.child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .px_4()
                .py_3()
                .border_t_1()
                .border_color(theme.border)
                .child(cancel_btn)
                .child(confirm_btn),
        );

        backdrop.child(dialog)
    }
}

impl RenderOnce for ConfirmDialog {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.title.clone())
            .unwrap_or_default();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Alertdialog)),
        });

        let global_theme = cx.theme();
        let theme = ConfirmDialogTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for ConfirmDialog {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
