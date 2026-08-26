//! Progress component
//!
//! Progress bars and indicators.

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::arc::arc_path;
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Bounds, Div, ElementId, FontWeight, Pixels, Rgba, SharedString, Window, canvas, div, px,
    relative,
};
use std::f32::consts::{PI, TAU};

/// Progress variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressVariant {
    /// Default blue
    #[default]
    Default,
    /// Success green
    Success,
    /// Warning yellow
    Warning,
    /// Error red
    Error,
}

impl ProgressVariant {
    fn color(&self, theme: &Theme) -> Rgba {
        match self {
            ProgressVariant::Default => theme.accent,
            ProgressVariant::Success => theme.success,
            ProgressVariant::Warning => theme.warning,
            ProgressVariant::Error => theme.error,
        }
    }
}

/// Progress size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressSize {
    /// Extra small (2px)
    Xs,
    /// Small (4px)
    Sm,
    /// Medium (8px, default)
    #[default]
    Md,
    /// Large (12px)
    Lg,
}

impl ProgressSize {
    fn height(&self) -> Pixels {
        match self {
            ProgressSize::Xs => px(2.0),
            ProgressSize::Sm => px(4.0),
            ProgressSize::Md => px(8.0),
            ProgressSize::Lg => px(12.0),
        }
    }
}

impl From<crate::ComponentSize> for ProgressSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

fn progress_percentage(value: f32, max: f32) -> f32 {
    if value.is_finite() && max.is_finite() && max > 0.0 {
        (value / max * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    }
}

/// A progress bar component
pub struct Progress {
    value: f32,
    max: f32,
    variant: ProgressVariant,
    size: ProgressSize,
    show_label: bool,
    striped: bool,
    animated: bool,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Progress {
    /// Create a new progress bar
    /// Value should be between 0.0 and 1.0 (or 0.0 to max if max is set)
    pub fn new(value: f32) -> Self {
        Self {
            value,
            max: 1.0,
            variant: ProgressVariant::default(),
            size: ProgressSize::default(),
            show_label: false,
            striped: false,
            animated: false,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set maximum value
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Set variant
    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size
    pub fn size(mut self, size: ProgressSize) -> Self {
        self.size = size;
        self
    }

    /// Show percentage label
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// Enable striped appearance
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Enable animation
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Progressbar)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let height = self.size.height();
        let color = self.variant.color(theme);
        let percentage = progress_percentage(self.value, self.max);

        let mut container = div().flex().flex_col().gap_1().w_full();

        // Label
        if self.show_label {
            container = container.child(
                div()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(format!("{:.0}%", percentage)),
            );
        }

        // Track
        let track = div()
            .w_full()
            .h(height)
            .bg(theme.surface)
            .rounded_full()
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .bg(color)
                    .rounded_full()
                    .w(relative(percentage / 100.0)),
            );

        container = container.child(track);

        container
    }
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        cx.register_accessible(AccessibilityNode {
            element_id: ElementId::Name("progress".into()),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Progressbar))
                .value_range(self.value as f64, 0.0, self.max as f64),
        });

        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Progress {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A circular progress indicator
pub struct CircularProgress {
    value: f32,
    max: f32,
    size: Pixels,
    thickness: Pixels,
    variant: ProgressVariant,
    show_label: bool,
}

impl CircularProgress {
    /// Create a new circular progress
    /// Value should be between 0.0 and 1.0 (or 0.0 to max if max is set)
    pub fn new(value: f32) -> Self {
        Self {
            value,
            max: 1.0,
            size: px(48.0),
            thickness: px(4.0),
            variant: ProgressVariant::default(),
            show_label: false,
        }
    }

    /// Set maximum value
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Set size
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    /// Set thickness
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = thickness;
        self
    }

    /// Set variant
    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Show percentage label in center
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// Build into element with theme.
    ///
    /// The ring is painted as a real track plus a value arc. This preserves
    /// the progress geometry instead of encoding the value only as a blended
    /// border color.
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let percentage = progress_percentage(self.value, self.max);
        let base_color = self.variant.color(theme);
        let progress_ratio = percentage / 100.0;
        let size = self.size;
        let thickness = self.thickness;
        let track_color = theme.surface;

        let ring = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds: Bounds<Pixels>, (), window, _cx| {
                if let Some(path) = arc_path(bounds, thickness, -PI / 2.0, TAU) {
                    window.paint_path(path, track_color);
                }
                if progress_ratio > 0.0
                    && let Some(path) = arc_path(bounds, thickness, -PI / 2.0, TAU * progress_ratio)
                {
                    window.paint_path(path, base_color);
                }
            },
        )
        .w(size)
        .h(size);

        let mut container = div()
            .flex()
            .items_center()
            .justify_center()
            .w(size)
            .h(size)
            .relative();

        container = container.child(ring);

        // Center label
        if self.show_label {
            container = container.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_secondary)
                    .child(format!("{:.0}%", percentage)),
            );
        }

        container
    }
}

impl RenderOnce for CircularProgress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for CircularProgress {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
