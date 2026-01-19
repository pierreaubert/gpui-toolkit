//! Progress component
//!
//! Progress bars and indicators.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::*;

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

/// A progress bar component
pub struct Progress {
    value: f32,
    max: f32,
    variant: ProgressVariant,
    size: ProgressSize,
    show_label: bool,
    striped: bool,
    animated: bool,
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

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let height = self.size.height();
        let color = self.variant.color(theme);
        let percentage = (self.value / self.max * 100.0).clamp(0.0, 100.0);

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

    /// Build into element with theme
    /// Note: True circular progress requires canvas/SVG rendering.
    /// This is a simplified box-based representation where color intensity
    /// increases with progress value.
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let percentage = (self.value / self.max * 100.0).clamp(0.0, 100.0);
        let base_color = self.variant.color(theme);

        // Interpolate color intensity based on progress (0% = surface color, 100% = full color)
        let progress_ratio = percentage / 100.0;
        let color = if percentage <= 0.0 {
            theme.surface
        } else {
            // Blend between surface and full color based on progress
            let r = theme.surface.r * (1.0 - progress_ratio) + base_color.r * progress_ratio;
            let g = theme.surface.g * (1.0 - progress_ratio) + base_color.g * progress_ratio;
            let b = theme.surface.b * (1.0 - progress_ratio) + base_color.b * progress_ratio;
            Rgba { r, g, b, a: 1.0 }
        };

        let mut container = div()
            .flex()
            .items_center()
            .justify_center()
            .w(self.size)
            .h(self.size)
            .rounded_full()
            .border(self.thickness)
            .border_color(color)
            .relative();

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
