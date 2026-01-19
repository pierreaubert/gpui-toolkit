//! Toast notification component
//!
//! Provides non-blocking notifications that appear temporarily.

use crate::theme::{Theme, ThemeExt, ThemeVariant};
use gpui::prelude::*;
use gpui::{Component, *};

/// Toast visual variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastVariant {
    /// Informational message (default)
    #[default]
    Info,
    /// Success message
    Success,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl ToastVariant {
    fn icon(&self) -> &'static str {
        match self {
            ToastVariant::Info => "i",
            ToastVariant::Success => "v",
            ToastVariant::Warning => "!",
            ToastVariant::Error => "x",
        }
    }

    fn colors(&self, theme: &Theme) -> (Rgba, Rgba, Rgba) {
        // Returns (background, border, icon_color)
        match theme.variant {
            ThemeVariant::Light => match self {
                ToastVariant::Info => (theme.surface, theme.info, theme.info),
                ToastVariant::Success => (rgb(0xdcfce7), theme.success, theme.success),
                ToastVariant::Warning => (rgb(0xfef3c7), theme.warning, theme.warning),
                ToastVariant::Error => (rgb(0xfee2e2), theme.error, theme.error),
            },
            // Dark, Midnight, Forest, BlackAndWhite all use dark-style backgrounds
            ThemeVariant::Dark
            | ThemeVariant::Midnight
            | ThemeVariant::Forest
            | ThemeVariant::BlackAndWhite => match self {
                ToastVariant::Info => (theme.surface, theme.info, theme.info),
                ToastVariant::Success => (rgb(0x1a3a1a), theme.success, theme.success),
                ToastVariant::Warning => (rgb(0x3a3a1a), theme.warning, theme.warning),
                ToastVariant::Error => (rgb(0x3a1a1a), theme.error, theme.error),
            },
        }
    }
}

/// Toast position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastPosition {
    /// Top right corner
    TopRight,
    /// Top left corner
    TopLeft,
    /// Bottom right corner (default)
    #[default]
    BottomRight,
    /// Bottom left corner
    BottomLeft,
    /// Top center
    TopCenter,
    /// Bottom center
    BottomCenter,
}

/// A single toast notification
pub struct Toast {
    id: ElementId,
    title: Option<SharedString>,
    message: SharedString,
    variant: ToastVariant,
    closeable: bool,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Duration in seconds before auto-dismiss (None = no auto-dismiss, default = 5.0)
    duration_secs: Option<f32>,
}

impl Toast {
    /// Default duration for auto-dismiss in seconds
    pub const DEFAULT_DURATION_SECS: f32 = 5.0;

    /// Create a new toast with a message (auto-dismisses after 5 seconds by default)
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: None,
            message: message.into(),
            variant: ToastVariant::default(),
            closeable: true,
            on_close: None,
            duration_secs: Some(Self::DEFAULT_DURATION_SECS),
        }
    }

    /// Set the toast title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the toast variant
    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set whether the toast is closeable
    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    /// Set the close handler
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Set the auto-dismiss duration in seconds (None = no auto-dismiss)
    pub fn duration_secs(mut self, duration: Option<f32>) -> Self {
        self.duration_secs = duration;
        self
    }

    /// Make this toast persistent (no auto-dismiss)
    pub fn persistent(mut self) -> Self {
        self.duration_secs = None;
        self
    }

    /// Get the duration in seconds (for timer management)
    pub fn get_duration_secs(&self) -> Option<f32> {
        self.duration_secs
    }

    /// Get the duration in milliseconds (for timer management)
    pub fn get_duration_ms(&self) -> Option<u64> {
        self.duration_secs.map(|s| (s * 1000.0) as u64)
    }

    /// Build the toast into an element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let (bg, border, icon_color) = self.variant.colors(theme);
        let icon = self.variant.icon();
        // Clone ID for use in close button (self.id is moved to toast container)
        let close_btn_id = self.id.clone();

        let mut toast = div()
            .id(self.id)
            .w(px(320.0))
            .flex()
            .items_start()
            .gap_3()
            .px_4()
            .py_3()
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .shadow_lg();

        // Icon
        toast = toast.child(
            div()
                .text_lg()
                .text_color(icon_color)
                .mt(px(2.0))
                .child(icon),
        );

        // Content area
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

        toast = toast.child(content);

        // Close button
        if self.closeable {
            let text_muted = theme.text_muted;
            let text_primary = theme.text_primary;
            if let Some(handler) = self.on_close {
                let handler_ptr: *const dyn Fn(&mut Window, &mut App) = handler.as_ref();
                toast = toast.child(
                    div()
                        .id((close_btn_id, "close"))
                        .text_sm()
                        .text_color(text_muted)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(text_primary))
                        .on_mouse_up(MouseButton::Left, move |_event, window, cx| unsafe {
                            (*handler_ptr)(window, cx);
                        })
                        .child("x"),
                );
                std::mem::forget(handler);
            }
        }

        toast
    }
}

impl IntoElement for Toast {
    type Element = Component<Self>;

    fn into_element(self) -> Self::Element {
        Component::new(self)
    }
}

impl RenderOnce for Toast {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

/// A container for positioning toasts on screen
#[derive(IntoElement)]
pub struct ToastContainer {
    position: ToastPosition,
    toasts: Vec<Toast>,
}

impl ToastContainer {
    /// Create a new toast container
    pub fn new(position: ToastPosition) -> Self {
        Self {
            position,
            toasts: Vec::new(),
        }
    }

    /// Add a toast to the container
    pub fn toast(mut self, toast: Toast) -> Self {
        self.toasts.push(toast);
        self
    }

    /// Add multiple toasts
    pub fn toasts(mut self, toasts: impl IntoIterator<Item = Toast>) -> Self {
        self.toasts.extend(toasts);
        self
    }

    /// Build the container into an element
    pub fn build(self) -> Div {
        let mut container = div().absolute().flex().flex_col().gap_2().p_4();

        // Position the container
        match self.position {
            ToastPosition::TopRight => {
                container = container.top_0().right_0();
            }
            ToastPosition::TopLeft => {
                container = container.top_0().left_0();
            }
            ToastPosition::BottomRight => {
                container = container.bottom_0().right_0();
            }
            ToastPosition::BottomLeft => {
                container = container.bottom_0().left_0();
            }
            ToastPosition::TopCenter => {
                container = container.top_0().left_0().right_0().items_center();
            }
            ToastPosition::BottomCenter => {
                container = container.bottom_0().left_0().right_0().items_center();
            }
        }

        for toast in self.toasts {
            container = container.child(toast);
        }

        container
    }
}

impl RenderOnce for ToastContainer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.build()
    }
}
