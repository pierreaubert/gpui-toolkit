//! StepIndicator component
//!
//! A step progress indicator showing the current position in a multi-step workflow.
//!
//! # Usage
//!
//! ```ignore
//! StepIndicator::new("setup-steps", vec![
//!     StepItem::new("Account").status(StepItemStatus::Completed),
//!     StepItem::new("Profile").status(StepItemStatus::Active),
//!     StepItem::new("Confirm").status(StepItemStatus::NotVisited),
//! ])
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, Pixels, Rgba, SharedString, Stateful, Window, div, px,
};

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepItemStatus {
    /// Not yet reached
    #[default]
    NotVisited,
    /// Currently active step
    Active,
    /// Successfully completed
    Completed,
    /// Step encountered an error
    Error,
}

/// Orientation for the step indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepOrientation {
    /// Steps laid out horizontally (default)
    #[default]
    Horizontal,
    /// Steps laid out vertically
    Vertical,
}

/// Size of the step indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepIndicatorSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl StepIndicatorSize {
    fn circle_size(&self) -> Pixels {
        match self {
            StepIndicatorSize::Sm => px(24.0),
            StepIndicatorSize::Md => px(32.0),
            StepIndicatorSize::Lg => px(40.0),
        }
    }

    fn font_size_small(&self) -> bool {
        matches!(self, StepIndicatorSize::Sm)
    }
}

/// Theme colors for step indicator
#[derive(Debug, Clone, ComponentTheme)]
pub struct StepIndicatorTheme {
    /// Default step background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub step_bg: Rgba,
    /// Default step border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub step_border: Rgba,
    /// Active step background
    #[theme(default = 0x007accff, from = accent)]
    pub active_bg: Rgba,
    /// Completed step background
    #[theme(default = 0x22c55eff, from = success)]
    pub completed_bg: Rgba,
    /// Error step background
    #[theme(default = 0xdc2626ff, from = error)]
    pub error_bg: Rgba,
    /// Step circle text/icon color (on colored bg)
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub step_text: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_text: Rgba,
    /// Muted label text color (not visited)
    #[theme(default = 0x666666ff, from = text_muted)]
    pub label_muted: Rgba,
    /// Connector line color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub connector: Rgba,
    /// Completed connector line color
    #[theme(default = 0x22c55eff, from = success)]
    pub connector_completed: Rgba,
}

/// A single step item
pub struct StepItem {
    label: SharedString,
    status: StepItemStatus,
    icon: Option<SharedString>,
}

impl StepItem {
    /// Create a new step item
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            status: StepItemStatus::default(),
            icon: None,
        }
    }

    /// Set the status
    pub fn status(mut self, status: StepItemStatus) -> Self {
        self.status = status;
        self
    }

    /// Set a custom icon (overrides step number and status icons)
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// A step indicator component
pub struct StepIndicator {
    id: ElementId,
    steps: Vec<StepItem>,
    orientation: StepOrientation,
    size: StepIndicatorSize,
    on_click: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
}

impl StepIndicator {
    /// Create a new step indicator
    pub fn new(id: impl Into<ElementId>, steps: Vec<StepItem>) -> Self {
        Self {
            id: id.into(),
            steps,
            orientation: StepOrientation::default(),
            size: StepIndicatorSize::default(),
            on_click: None,
        }
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: StepOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set size
    pub fn size(mut self, size: StepIndicatorSize) -> Self {
        self.size = size;
        self
    }

    /// Called when a step is clicked (receives step index)
    pub fn on_click(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Build the step indicator with theme
    pub fn build_with_theme(self, theme: &StepIndicatorTheme) -> Stateful<Div> {
        let circle_size = self.size.circle_size();
        let small_font = self.size.font_size_small();
        let step_count = self.steps.len();

        let mut container = div().id(self.id).flex().items_center();

        container = match self.orientation {
            StepOrientation::Horizontal => container.flex_row().gap_0(),
            StepOrientation::Vertical => container.flex_col().gap_0(),
        };

        for (i, step) in self.steps.into_iter().enumerate() {
            let (bg_color, text_color, label_color) = match step.status {
                StepItemStatus::NotVisited => (theme.step_bg, theme.label_muted, theme.label_muted),
                StepItemStatus::Active => (theme.active_bg, theme.step_text, theme.label_text),
                StepItemStatus::Completed => {
                    (theme.completed_bg, theme.step_text, theme.label_text)
                }
                StepItemStatus::Error => (theme.error_bg, theme.step_text, theme.label_text),
            };

            // Step icon
            let step_icon = if step.status == StepItemStatus::Completed {
                SharedString::from("\u{2713}")
            } else if step.status == StepItemStatus::Error {
                SharedString::from("\u{2717}")
            } else if let Some(icon) = step.icon {
                icon
            } else {
                SharedString::from(format!("{}", i + 1))
            };

            // Circle
            let mut circle = div()
                .w(circle_size)
                .h(circle_size)
                .rounded_full()
                .bg(bg_color)
                .flex()
                .items_center()
                .justify_center()
                .text_color(text_color)
                .font_weight(FontWeight::BOLD)
                .flex_shrink_0();

            if step.status == StepItemStatus::NotVisited {
                circle = circle.border_1().border_color(theme.step_border);
            }

            if small_font {
                circle = circle.text_xs();
            } else {
                circle = circle.text_sm();
            }

            circle = circle.child(step_icon);

            // Step container with circle + label
            let mut step_el = div().flex().items_center().gap_2();
            step_el = match self.orientation {
                StepOrientation::Horizontal => step_el.flex_col(),
                StepOrientation::Vertical => step_el.flex_row(),
            };

            step_el = step_el.child(circle);

            // Label
            let mut label_el = div().text_color(label_color).child(step.label);

            label_el = label_el.text_xs();

            step_el = step_el.child(label_el);
            container = container.child(step_el);

            // Connector line (between steps, not after last)
            if i < step_count - 1 {
                let connector_color = if step.status == StepItemStatus::Completed {
                    theme.connector_completed
                } else {
                    theme.connector
                };

                let connector = match self.orientation {
                    StepOrientation::Horizontal => div()
                        .h(px(2.0))
                        .flex_1()
                        .min_w(px(20.0))
                        .mx_2()
                        .bg(connector_color),
                    StepOrientation::Vertical => div()
                        .w(px(2.0))
                        .flex_1()
                        .min_h(px(20.0))
                        .my_2()
                        .bg(connector_color),
                };

                container = container.child(connector);
            }
        }

        container
    }
}

impl RenderOnce for StepIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = StepIndicatorTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for StepIndicator {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
