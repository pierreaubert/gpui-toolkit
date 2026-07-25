//! ImageView component
//!
//! An image display component with sizing, fit modes, and fallback placeholder.
//!
//! # Usage
//!
//! ```ignore
//! ImageView::new("album-art")
//!     .src("path/to/image.png")
//!     .fit(ImageFit::Cover)
//!     .rounded(px(8.0))
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    StyledImage,
};
use gpui::{
    App, Div, ElementId, MouseButton, ObjectFit, Pixels, Rgba, SharedString, Stateful, Window, div,
    img,
};
use std::path::PathBuf;

/// How the image should fit within its container
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFit {
    /// Scale to fill while maintaining aspect ratio, may crop
    #[default]
    Cover,
    /// Scale to fit within bounds, may have letterboxing
    Contain,
    /// Stretch to fill exactly
    Fill,
}

impl From<ImageFit> for ObjectFit {
    fn from(value: ImageFit) -> Self {
        match value {
            ImageFit::Cover => Self::Cover,
            ImageFit::Contain => Self::Contain,
            ImageFit::Fill => Self::Fill,
        }
    }
}

/// Theme colors for image view styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ImageViewTheme {
    /// Placeholder background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub placeholder_bg: Rgba,
    /// Placeholder icon/text color
    #[theme(default = 0x555555ff, from = text_muted)]
    pub placeholder_text: Rgba,
    /// Border color (when border is enabled)
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
}

/// An image display component
pub struct ImageView {
    id: ElementId,
    src: Option<SharedString>,
    alt: SharedString,
    width: Option<Pixels>,
    height: Option<Pixels>,
    fit: ImageFit,
    border_radius: Option<Pixels>,
    show_border: bool,
    placeholder_icon: SharedString,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl ImageView {
    /// Create a new image view
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            src: None,
            alt: "".into(),
            width: None,
            height: None,
            fit: ImageFit::default(),
            border_radius: None,
            show_border: false,
            placeholder_icon: "🖼".into(),
            on_click: None,
        }
    }

    /// Set the image source path
    pub fn src(mut self, src: impl Into<SharedString>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Set alt text
    pub fn alt(mut self, alt: impl Into<SharedString>) -> Self {
        self.alt = alt.into();
        self
    }

    /// Set width
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Set height
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Set both width and height
    pub fn size(mut self, size: Pixels) -> Self {
        self.width = Some(size);
        self.height = Some(size);
        self
    }

    /// Set image fit mode
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set border radius for rounded corners
    pub fn rounded(mut self, radius: Pixels) -> Self {
        self.border_radius = Some(radius);
        self
    }

    /// Show a border around the image
    pub fn show_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// Set the placeholder icon displayed when no image is set
    pub fn placeholder_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.placeholder_icon = icon.into();
        self
    }

    /// Set click handler
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Build the image view with theme
    pub fn build_with_theme(self, theme: &ImageViewTheme) -> Stateful<Div> {
        let image_fit = self.fit.into();
        let placeholder_icon = self.placeholder_icon.clone();
        let mut container = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden();

        if let Some(w) = self.width {
            container = container.w(w);
        }
        if let Some(h) = self.height {
            container = container.h(h);
        }

        if let Some(radius) = self.border_radius {
            container = container.rounded(radius);
        }

        if self.show_border {
            container = container.border_1().border_color(theme.border);
        }

        if !self.alt.is_empty() {
            container = container.aria_label(self.alt);
        }

        container = container.bg(theme.placeholder_bg);
        if let Some(source) = self.src {
            let mut image = if source.contains("://") || source.starts_with("data:") {
                img(source)
            } else {
                img(PathBuf::from(source.as_ref()))
            };
            let fallback_icon = placeholder_icon.clone();
            let fallback_color = theme.placeholder_text;
            image = image
                .size_full()
                .object_fit(image_fit)
                .with_fallback(move || {
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .text_color(fallback_color)
                        .child(fallback_icon.clone())
                        .into_any_element()
                });
            container = container.child(image);
        } else {
            container = container.child(
                div()
                    .text_3xl()
                    .text_color(theme.placeholder_text)
                    .child(placeholder_icon),
            );
        }

        if let Some(handler) = self.on_click {
            container = container.cursor_pointer().on_mouse_up(
                MouseButton::Left,
                move |_event, window, cx| {
                    handler(window, cx);
                },
            );
        }

        container
    }
}

impl RenderOnce for ImageView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = ImageViewTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for ImageView {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::ImageFit;
    use gpui::ObjectFit;

    #[test]
    fn image_fit_maps_to_gpui_object_fit() {
        assert!(matches!(ObjectFit::from(ImageFit::Cover), ObjectFit::Cover));
        assert!(matches!(
            ObjectFit::from(ImageFit::Contain),
            ObjectFit::Contain
        ));
        assert!(matches!(ObjectFit::from(ImageFit::Fill), ObjectFit::Fill));
    }
}
