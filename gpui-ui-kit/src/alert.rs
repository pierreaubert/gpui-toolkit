//! Alert component
//!
//! Contextual feedback messages.

use crate::theme::{Theme, ThemeExt, ThemeVariant};
use gpui::prelude::*;
use gpui::{Component, *};

/// Alert variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertVariant {
    /// Informational (default)
    #[default]
    Info,
    /// Success message
    Success,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl AlertVariant {
    fn colors(&self, theme: &Theme) -> (Rgba, Rgba, Rgba) {
        // Returns (background, border, icon_color)
        match theme.variant {
            ThemeVariant::Light => match self {
                AlertVariant::Info => (rgb(0xe0f2fe), theme.info, theme.info),
                AlertVariant::Success => (rgb(0xdcfce7), theme.success, theme.success),
                AlertVariant::Warning => (rgb(0xfef3c7), theme.warning, theme.warning),
                AlertVariant::Error => (rgb(0xfee2e2), theme.error, theme.error),
            },
            // Dark, Midnight, Forest, BlackAndWhite all use dark-style backgrounds
            ThemeVariant::Dark
            | ThemeVariant::Midnight
            | ThemeVariant::Forest
            | ThemeVariant::BlackAndWhite => match self {
                AlertVariant::Info => (rgb(0x1a2a3a), theme.info, theme.info),
                AlertVariant::Success => (rgb(0x1a3a1a), theme.success, theme.success),
                AlertVariant::Warning => (rgb(0x3a3a1a), theme.warning, theme.warning),
                AlertVariant::Error => (rgb(0x3a1a1a), theme.error, theme.error),
            },
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            AlertVariant::Info => "i",
            AlertVariant::Success => "v",
            AlertVariant::Warning => "!",
            AlertVariant::Error => "x",
        }
    }
}

/// An alert component
pub struct Alert {
    id: ElementId,
    title: Option<SharedString>,
    message: SharedString,
    variant: AlertVariant,
    closeable: bool,
    icon: Option<SharedString>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Alert {
    /// Create a new alert
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: None,
            message: message.into(),
            variant: AlertVariant::default(),
            closeable: false,
            icon: None,
            on_close: None,
        }
    }

    /// Set title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set variant
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Make closeable
    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    /// Set custom icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set close handler
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let (bg, border, icon_color) = self.variant.colors(theme);
        let default_icon = self.variant.icon();
        // Clone ID for use in close button (self.id is moved to alert container)
        let close_btn_id = self.id.clone();

        let mut alert = div()
            .id(self.id)
            .flex()
            .items_start()
            .gap_3()
            .p_4()
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg();

        // Icon
        let icon = self.icon.unwrap_or_else(|| default_icon.into());
        alert = alert.child(div().text_lg().text_color(icon_color).child(icon));

        // Content
        let mut content = div().flex_1().flex().flex_col().gap_1();

        if let Some(title) = self.title {
            content = content.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child(title),
            );
        }

        content = content.child(
            div()
                .text_sm()
                .text_color(theme.text_secondary)
                .child(self.message),
        );

        alert = alert.child(content);

        // Close button
        if self.closeable {
            let text_muted = theme.text_muted;
            let text_primary = theme.text_primary;
            let mut close_btn = div()
                .id((close_btn_id, "close"))
                .text_sm()
                .text_color(text_muted)
                .cursor_pointer()
                .hover(move |s| s.text_color(text_primary));

            if let Some(handler) = self.on_close {
                let handler_ptr: *const dyn Fn(&mut Window, &mut App) = handler.as_ref();
                close_btn =
                    close_btn.on_mouse_up(MouseButton::Left, move |_event, window, cx| unsafe {
                        (*handler_ptr)(window, cx);
                    });
                std::mem::forget(handler);
            }

            alert = alert.child(close_btn.child("x"));
        }

        alert
    }
}

impl IntoElement for Alert {
    type Element = Component<Self>;

    fn into_element(self) -> Self::Element {
        Component::new(self)
    }
}

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

/// A simple inline alert (no close button)
#[derive(IntoElement)]
pub struct InlineAlert {
    message: SharedString,
    variant: AlertVariant,
}

impl InlineAlert {
    /// Create a new inline alert
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
            variant: AlertVariant::default(),
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let (_, _border, icon_color) = self.variant.colors(theme);
        let icon = self.variant.icon();

        div()
            .flex()
            .items_center()
            .gap_2()
            .text_sm()
            .text_color(icon_color)
            .child(div().child(icon))
            .child(self.message)
    }
}

impl RenderOnce for InlineAlert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}
