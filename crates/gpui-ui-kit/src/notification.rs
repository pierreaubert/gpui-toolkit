//! Notification / Banner component
//!
//! Persistent top-of-page notifications that stay visible until dismissed.
//! Unlike toasts (transient), banners persist and require explicit dismissal.
//!
//! # Usage
//!
//! ```ignore
//! Notification::new("wallet-banner", "Wallet connected")
//!     .variant(NotificationVariant::Success)
//!     .description("Your wallet is ready to use")
//!     .action("Disconnect", |window, cx| { /* handle */ })
//!     .dismissible(true)
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};

/// Notification variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationVariant {
    /// Informational (default)
    #[default]
    Info,
    /// Success
    Success,
    /// Warning
    Warning,
    /// Error
    Error,
}

/// Theme colors for notification
#[derive(Debug, Clone, ComponentTheme)]
pub struct NotificationTheme {
    /// Info background
    #[theme(default = 0x1a2a3aff, from = alert_info_bg)]
    pub info_bg: Rgba,
    /// Info accent/icon color
    #[theme(default = 0x60a5faff, from = info)]
    pub info_accent: Rgba,
    /// Success background
    #[theme(default = 0x1a3a2aff, from = alert_success_bg)]
    pub success_bg: Rgba,
    /// Success accent
    #[theme(default = 0x4ade80ff, from = success)]
    pub success_accent: Rgba,
    /// Warning background
    #[theme(default = 0x3a3a1aff, from = alert_warning_bg)]
    pub warning_bg: Rgba,
    /// Warning accent
    #[theme(default = 0xfbbf24ff, from = warning)]
    pub warning_accent: Rgba,
    /// Error background
    #[theme(default = 0x3a1a1aff, from = alert_error_bg)]
    pub error_bg: Rgba,
    /// Error accent
    #[theme(default = 0xf87171ff, from = error)]
    pub error_accent: Rgba,
    /// Text color
    #[theme(default = 0xeeeeeeff, from = text_primary)]
    pub text: Rgba,
    /// Description text color
    #[theme(default = 0xaaaaaabb, from = text_secondary)]
    pub text_secondary: Rgba,
    /// Dismiss button hover
    #[theme(default = 0x55555544, from = surface_hover)]
    pub dismiss_hover: Rgba,
}

/// A persistent notification/banner component
pub struct Notification {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    variant: NotificationVariant,
    icon: Option<SharedString>,
    dismissible: bool,
    on_dismiss: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    action_label: Option<SharedString>,
    action_handler: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Notification {
    /// Create a new notification
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            variant: NotificationVariant::default(),
            icon: None,
            dismissible: true,
            on_dismiss: None,
            action_label: None,
            action_handler: None,
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: NotificationVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set description text
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set custom icon (overrides variant icon)
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Allow dismissing
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Called when dismissed
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    /// Add an action button
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.action_label = Some(label.into());
        self.action_handler = Some(Box::new(handler));
        self
    }

    /// Build with theme
    pub fn build_with_theme(self, theme: &NotificationTheme) -> Stateful<Div> {
        let (bg, accent) = match self.variant {
            NotificationVariant::Info => (theme.info_bg, theme.info_accent),
            NotificationVariant::Success => (theme.success_bg, theme.success_accent),
            NotificationVariant::Warning => (theme.warning_bg, theme.warning_accent),
            NotificationVariant::Error => (theme.error_bg, theme.error_accent),
        };

        let default_icon = match self.variant {
            NotificationVariant::Info => "i",
            NotificationVariant::Success => "v",
            NotificationVariant::Warning => "!",
            NotificationVariant::Error => "x",
        };

        let dismiss_id = (self.id.clone(), "dismiss");
        let action_id = (self.id.clone(), "action");

        let mut banner = div()
            .id(self.id)
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .bg(bg)
            .border_l_4()
            .border_color(accent);

        // Icon
        let icon_text = self.icon.unwrap_or_else(|| default_icon.into());
        banner = banner.child(
            div()
                .text_color(accent)
                .font_weight(FontWeight::BOLD)
                .child(icon_text),
        );

        // Content
        let mut content = div().flex_1().flex().flex_col().gap(px(2.0));
        content = content.child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text)
                .child(self.title),
        );
        if let Some(desc) = self.description {
            content = content.child(div().text_xs().text_color(theme.text_secondary).child(desc));
        }
        banner = banner.child(content);

        // Action button
        if let Some(label) = self.action_label {
            let mut action_btn = div()
                .id(action_id)
                .px_3()
                .py_1()
                .rounded(px(4.0))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(accent)
                .cursor_pointer()
                .border_1()
                .border_color(accent);

            if let Some(handler) = self.action_handler {
                action_btn =
                    action_btn.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                        handler(window, cx);
                    });
            }

            action_btn = action_btn.child(label);
            banner = banner.child(action_btn);
        }

        // Dismiss button
        if self.dismissible {
            let dismiss_hover = theme.dismiss_hover;
            let mut dismiss_btn = div()
                .id(dismiss_id)
                .cursor_pointer()
                .p_1()
                .rounded(px(4.0))
                .text_color(theme.text_secondary)
                .hover(move |s| s.bg(dismiss_hover))
                .child("\u{2715}"); // ✕

            if let Some(handler) = self.on_dismiss {
                dismiss_btn =
                    dismiss_btn.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                        handler(window, cx);
                    });
            }
            banner = banner.child(dismiss_btn);
        }

        banner
    }
}

impl RenderOnce for Notification {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = NotificationTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Notification {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
