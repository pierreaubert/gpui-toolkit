//! StatusBar component
//!
//! A horizontal bar for displaying status information, typically at the top or bottom
//! of an application window.
//!
//! # Usage
//!
//! ```ignore
//! StatusBar::new("footer")
//!     .position(StatusBarPosition::Bottom)
//!     .left(playback_controls)
//!     .center(track_info)
//!     .right(volume_control)
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{AnyElement, App, Div, ElementId, Pixels, Rgba, Stateful, Window, div, px};

/// Position of the status bar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBarPosition {
    /// Top of the window
    Top,
    /// Bottom of the window (default)
    #[default]
    Bottom,
}

/// Theme colors for status bar styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct StatusBarTheme {
    /// Background color
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Primary text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Muted text color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub text_muted: Rgba,
}

/// A status bar component with left, center, and right sections
pub struct StatusBar {
    id: ElementId,
    position: StatusBarPosition,
    left: Option<AnyElement>,
    center: Option<AnyElement>,
    right: Option<AnyElement>,
    height: Pixels,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            position: StatusBarPosition::default(),
            left: None,
            center: None,
            right: None,
            height: px(32.0),
        }
    }

    /// Set the position
    pub fn position(mut self, position: StatusBarPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the left section content
    pub fn left(mut self, element: impl IntoElement) -> Self {
        self.left = Some(element.into_any_element());
        self
    }

    /// Set the center section content
    pub fn center(mut self, element: impl IntoElement) -> Self {
        self.center = Some(element.into_any_element());
        self
    }

    /// Set the right section content
    pub fn right(mut self, element: impl IntoElement) -> Self {
        self.right = Some(element.into_any_element());
        self
    }

    /// Set height
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    /// Build the status bar with theme
    pub fn build_with_theme(self, theme: &StatusBarTheme) -> Stateful<Div> {
        let mut bar = div()
            .id(self.id)
            .w_full()
            .h(self.height)
            .flex_shrink_0()
            .flex()
            .items_center()
            .px_3()
            .bg(theme.background)
            .text_xs()
            .text_color(theme.text);

        // Border on the appropriate edge
        bar = match self.position {
            StatusBarPosition::Top => bar.border_b_1().border_color(theme.border),
            StatusBarPosition::Bottom => bar.border_t_1().border_color(theme.border),
        };

        // Left section
        bar = bar.child(div().flex().items_center().flex_1().children(self.left));

        // Center section
        bar = bar.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_1()
                .children(self.center),
        );

        // Right section
        bar = bar.child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .flex_1()
                .children(self.right),
        );

        bar
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = StatusBarTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for StatusBar {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
