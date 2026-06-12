//! LoadingOverlay component
//!
//! A full-area overlay with a spinner and optional message, used to indicate
//! loading or processing state.
//!
//! # Usage
//!
//! ```ignore
//! LoadingOverlay::new("loading")
//!     .message("Loading library...")
//!     .spinner_size(SpinnerSize::Lg)
//! ```

use crate::ComponentTheme;
use crate::spinner::{Spinner, SpinnerSize};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, MouseButton, Rgba, SharedString, Stateful, Window, div,
};

/// Theme colors for loading overlay
#[derive(Debug, Clone, ComponentTheme)]
pub struct LoadingOverlayTheme {
    /// Backdrop background
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
    /// Message text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text: Rgba,
    /// Secondary text color (for subtitle)
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text_secondary: Rgba,
}

/// A loading overlay component
pub struct LoadingOverlay {
    id: ElementId,
    message: Option<SharedString>,
    subtitle: Option<SharedString>,
    spinner_size: SpinnerSize,
    spinner_color: Option<Rgba>,
    dismissible: bool,
    on_dismiss: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl LoadingOverlay {
    /// Create a new loading overlay
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            message: None,
            subtitle: None,
            spinner_size: SpinnerSize::Lg,
            spinner_color: None,
            dismissible: false,
            on_dismiss: None,
        }
    }

    /// Set the loading message
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set a subtitle below the message
    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set spinner size
    pub fn spinner_size(mut self, size: SpinnerSize) -> Self {
        self.spinner_size = size;
        self
    }

    /// Set spinner color
    pub fn spinner_color(mut self, color: Rgba) -> Self {
        self.spinner_color = Some(color);
        self
    }

    /// Allow dismissing the overlay by clicking
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Called when the overlay is dismissed
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    /// Build the loading overlay with theme
    pub fn build_with_theme(self, theme: &LoadingOverlayTheme) -> Stateful<Div> {
        let mut overlay = div()
            .id(self.id)
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(theme.backdrop)
            // Block scroll and click-through
            .on_scroll_wheel(|_event, _window, _cx| {})
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {});

        if let (true, Some(handler)) = (self.dismissible, self.on_dismiss) {
            overlay = overlay.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler(window, cx);
            });
        }

        // Spinner
        let mut spinner = Spinner::new().size(self.spinner_size);
        if let Some(color) = self.spinner_color {
            spinner = spinner.color(color);
        }
        overlay = overlay.child(spinner);

        // Message
        if let Some(message) = self.message {
            overlay = overlay.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child(message),
            );
        }

        // Subtitle
        if let Some(subtitle) = self.subtitle {
            overlay = overlay.child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(subtitle),
            );
        }

        overlay
    }
}

impl RenderOnce for LoadingOverlay {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = LoadingOverlayTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for LoadingOverlay {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
