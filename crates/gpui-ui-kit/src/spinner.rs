//! Spinner component
//!
//! Loading indicators and spinners.

use crate::arc::arc_path;
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    Animation, AnimationExt, App, Div, ElementId, Pixels, Rgba, SharedString, Window, canvas, div,
    px,
};
use std::cell::Cell;
use std::f32::consts::{PI, TAU};
use std::rc::Rc;
use std::time::Duration;

/// Spinner size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    /// Extra small (12px)
    Xs,
    /// Small (16px)
    Sm,
    /// Medium (24px, default)
    #[default]
    Md,
    /// Large (32px)
    Lg,
    /// Extra large (48px)
    Xl,
}

impl SpinnerSize {
    fn size(&self) -> Pixels {
        match self {
            SpinnerSize::Xs => px(12.0),
            SpinnerSize::Sm => px(16.0),
            SpinnerSize::Md => px(24.0),
            SpinnerSize::Lg => px(32.0),
            SpinnerSize::Xl => px(48.0),
        }
    }

    fn border_width(&self) -> Pixels {
        match self {
            SpinnerSize::Xs => px(1.5),
            SpinnerSize::Sm => px(2.0),
            SpinnerSize::Md => px(2.5),
            SpinnerSize::Lg => px(3.0),
            SpinnerSize::Xl => px(4.0),
        }
    }
}

impl From<crate::ComponentSize> for SpinnerSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg => Self::Lg,
            crate::ComponentSize::Xl => Self::Xl,
        }
    }
}

/// A spinner/loading indicator component.
///
/// The indicator continuously repaints a moving arc through GPUI's native
/// animation-frame support.
pub struct Spinner {
    size: SpinnerSize,
    color: Option<Rgba>,
    label: Option<SharedString>,
}

impl Spinner {
    /// Create a new spinner
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::default(),
            color: None,
            label: None,
        }
    }

    /// Set size
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Set loading label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let size = self.size.size();
        let border_width = self.size.border_width();
        let color = self.color.unwrap_or(theme.accent);
        let track_color = theme.surface;

        let mut container = div().flex().items_center().gap_2();

        let phase = Rc::new(Cell::new(0.0_f32));
        let phase_for_paint = phase.clone();
        let spinner = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds, (), window, _cx| {
                let phase = phase_for_paint.get();
                if let Some(path) = arc_path(bounds, border_width, -PI / 2.0, TAU) {
                    window.paint_path(path, track_color);
                }
                if let Some(path) =
                    arc_path(bounds, border_width, phase * TAU - PI / 2.0, PI * 1.35)
                {
                    window.paint_path(path, color);
                }
            },
        )
        .w(size)
        .h(size)
        .with_animation(
            ElementId::from(("spinner-animation", 0_u32)),
            Animation::new(Duration::from_millis(900)).repeat(),
            move |element, delta| {
                phase.set(delta);
                element
            },
        );

        container = container.child(spinner);

        // Label
        if let Some(label) = self.label {
            let label_el = match self.size {
                SpinnerSize::Xs | SpinnerSize::Sm => div().text_xs(),
                SpinnerSize::Md => div().text_sm(),
                SpinnerSize::Lg | SpinnerSize::Xl => div(),
            };
            container = container.child(label_el.text_color(theme.text_secondary).child(label));
        }

        container
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Spinner {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A dots loading indicator
pub struct LoadingDots {
    size: SpinnerSize,
    color: Option<Rgba>,
}

impl LoadingDots {
    /// Create new loading dots
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::default(),
            color: None,
        }
    }

    /// Set size
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let color = self.color.unwrap_or(theme.accent);
        let dot_size = match self.size {
            SpinnerSize::Xs => px(4.0),
            SpinnerSize::Sm => px(6.0),
            SpinnerSize::Md => px(8.0),
            SpinnerSize::Lg => px(10.0),
            SpinnerSize::Xl => px(12.0),
        };

        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().w(dot_size).h(dot_size).rounded_full().bg(color))
            .child(
                div()
                    .w(dot_size)
                    .h(dot_size)
                    .rounded_full()
                    .bg(color)
                    .opacity(0.7),
            )
            .child(
                div()
                    .w(dot_size)
                    .h(dot_size)
                    .rounded_full()
                    .bg(color)
                    .opacity(0.4),
            )
    }
}

impl Default for LoadingDots {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for LoadingDots {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for LoadingDots {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
