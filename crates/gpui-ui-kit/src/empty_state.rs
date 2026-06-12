//! EmptyState component
//!
//! A placeholder displayed when a list or container has no content.
//!
//! # Usage
//!
//! ```ignore
//! EmptyState::new("No albums found")
//!     .description("Try adjusting your search filters")
//!     .icon("📂")
//! ```

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{AnyElement, App, Div, FontWeight, SharedString, Window, div};

/// An empty state placeholder component
#[derive(IntoElement)]
pub struct EmptyState {
    title: SharedString,
    description: Option<SharedString>,
    icon: Option<SharedString>,
    action: Option<AnyElement>,
}

impl EmptyState {
    /// Create a new empty state
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            action: None,
        }
    }

    /// Set a description
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set an icon (text/emoji)
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set an action element (e.g., a button)
    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.action = Some(element.into_any_element());
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let mut container = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .py_8()
            .gap_2();

        // Icon
        if let Some(icon) = self.icon {
            container = container.child(
                div()
                    .text_3xl()
                    .text_color(theme.text_muted)
                    .mb_2()
                    .child(icon),
            );
        }

        // Title
        container = container.child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(self.title),
        );

        // Description
        if let Some(desc) = self.description {
            container = container.child(div().text_xs().text_color(theme.text_muted).child(desc));
        }

        // Action
        if let Some(action) = self.action {
            container = container.child(div().mt_3().child(action));
        }

        container
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}
