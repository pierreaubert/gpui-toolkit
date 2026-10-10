//! Toast notification component
//!
//! Provides non-blocking notifications that appear temporarily.

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaLive, AriaProps, AriaRole};
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Component, Div, ElementId, FontWeight, MouseButton, Rgba, SharedString, Stateful, Window,
    div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

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

    pub fn colors(&self, theme: &Theme) -> (Rgba, Rgba, Rgba) {
        // Returns (background, border, icon_color)
        match self {
            ToastVariant::Info => (theme.surface, theme.info, theme.info),
            ToastVariant::Success => (theme.alert_success_bg, theme.success, theme.success),
            ToastVariant::Warning => (theme.alert_warning_bg, theme.warning, theme.warning),
            ToastVariant::Error => (theme.alert_error_bg, theme.error, theme.error),
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
    /// Display duration exposed to the toast host (`None` means persistent,
    /// default = 5.0 seconds).
    duration_secs: Option<f32>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
    design: Option<Arc<DesignSystem>>,
}

impl Toast {
    /// Default display duration in seconds.
    pub const DEFAULT_DURATION_SECS: f32 = 5.0;

    /// Create a toast with a five-second default display duration.
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: None,
            message: message.into(),
            variant: ToastVariant::default(),
            closeable: true,
            on_close: None,
            duration_secs: Some(Self::DEFAULT_DURATION_SECS),
            aria_label: None,
            aria_role: None,
            design: None,
        }
    }

    /// Set the design system (falls back to the app-global design when unset)
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
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

    /// Set an explicit ARIA label (overrides the toast's message)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Status)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Set the display duration exposed to the toast host (`None` = persistent).
    pub fn duration_secs(mut self, duration: Option<f32>) -> Self {
        self.duration_secs = duration;
        self
    }

    /// Make this toast persistent (no host-scheduled dismissal).
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

    /// Build the toast into an element with theme (uses the neutral design for geometry)
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the toast into an element with explicit theme and design
    pub fn build_with_theme_and_design(
        self,
        theme: &Theme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let (bg, border, icon_color) = self.variant.colors(theme);
        let icon = self.variant.icon();
        // Clone ID for use in close button (self.id is moved to toast container)
        let close_btn_id = self.id.clone();

        let mut toast = div()
            .id(self.id)
            .w(px(320.0))
            .flex()
            .items_start()
            .gap(px(design.spacing.control_gap * 1.5))
            .px(px(design.spacing.section_gap))
            .py(px(design.spacing.control_gap * 1.5))
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded(px(design.corners.md))
            .shadow_lg();

        // Icon
        toast = toast.child(
            div()
                .text_size(px(design.typography.large_size))
                .text_color(icon_color)
                .mt(px(design.spacing.grid_unit * 0.5))
                .child(icon),
        );

        // Content area
        let mut content = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(design.spacing.grid_unit));

        if let Some(title) = self.title {
            content = content.child(
                div()
                    .text_size(px(design.typography.base_size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child(title),
            );
        }

        content = content.child(
            div()
                .text_size(px(design.typography.base_size))
                .text_color(theme.text_secondary)
                .child(self.message),
        );

        toast = toast.child(content);

        // Close button
        if self.closeable {
            let text_muted = theme.text_muted;
            let text_primary = theme.text_primary;
            if let Some(handler) = self.on_close {
                let handler_rc = std::rc::Rc::new(handler);
                toast = toast.child(
                    div()
                        .id((close_btn_id, "close"))
                        .text_size(px(design.typography.base_size))
                        .text_color(text_muted)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(text_primary))
                        .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                            handler_rc(window, cx);
                        })
                        .child("x"),
                );
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
        let effective_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| self.message.clone());
        let (default_role, live) = match self.variant {
            ToastVariant::Error => (AriaRole::Alert, AriaLive::Assertive),
            ToastVariant::Warning => (AriaRole::Alert, AriaLive::Polite),
            _ => (AriaRole::Status, AriaLive::Polite),
        };
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(default_role)).live(live),
        });

        let design = crate::design::resolve_design(self.design.clone(), cx);
        let theme = cx.theme();
        self.build_with_theme_and_design(&theme, &design)
    }
}

/// A container for positioning toasts on screen
#[derive(IntoElement)]
pub struct ToastContainer {
    position: ToastPosition,
    toasts: Vec<Toast>,
    design: Option<Arc<DesignSystem>>,
}

impl ToastContainer {
    /// Create a new toast container
    pub fn new(position: ToastPosition) -> Self {
        Self {
            position,
            toasts: Vec::new(),
            design: None,
        }
    }

    /// Set the design system (falls back to the app-global design when unset)
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
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

    /// Build the container into an element (uses the neutral design for geometry)
    pub fn build(self) -> Div {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_design(&design)
    }

    /// Build the container into an element with an explicit design.
    /// Toasts without their own explicit design inherit the container's.
    pub fn build_with_design(self, design: &Arc<DesignSystem>) -> Div {
        let mut container = div()
            .absolute()
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap))
            .p(px(design.spacing.section_gap));

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

        for mut toast in self.toasts {
            if toast.design.is_none() {
                toast.design = Some(Arc::clone(design));
            }
            container = container.child(toast);
        }

        container
    }
}

impl RenderOnce for ToastContainer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_design(&design)
    }
}
