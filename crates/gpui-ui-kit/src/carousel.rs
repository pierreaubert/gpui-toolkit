//! `Carousel` component
//!
//! A parent-owned slide rotation: the caller holds the slide list and the
//! active index, while the carousel renders the active slide with previous,
//! next, and dot navigation. Index changes are reported through `on_change`
//! so the parent can store the new selection.
//!
//! # Usage
//!
//! ```ignore
//! Carousel::new("tour")
//!     .slide(CarouselSlide::new("Fast", "Starts in milliseconds."))
//!     .slide(CarouselSlide::new("Portable", "Runs on every target."))
//!     .index(current)
//!     .on_change(|next, _window, _cx| {
//!         // Store `next` as the new index.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, ClickEvent, Div, ElementId, KeyDownEvent, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// One titled slide with a short body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarouselSlide {
    /// Slide heading shown above the body.
    pub title: SharedString,
    /// Slide body text.
    pub body: SharedString,
}

impl CarouselSlide {
    /// Create a slide from `title` with `body`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let slide = CarouselSlide::new("Fast", "Starts in milliseconds.");
    /// ```
    pub fn new(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// Theme colors for the carousel.
#[derive(Debug, Clone, ComponentTheme)]
pub struct CarouselTheme {
    /// Slide panel background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub panel: Rgba,
    /// Slide title color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub title: Rgba,
    /// Slide body color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub body: Rgba,
    /// Inactive dot and panel border color.
    #[theme(default = 0x3e3e42ff, from = border)]
    pub dot: Rgba,
    /// Active dot color.
    #[theme(default = 0x007accff, from = accent)]
    pub dot_active: Rgba,
    /// Previous/next control color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub control: Rgba,
}

/// Carousel size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarouselSize {
    /// Compact slides.
    Sm,
    /// Standard slides (default).
    #[default]
    Md,
    /// Large slides.
    Lg,
}

impl From<crate::ComponentSize> for CarouselSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Slide rotation with parent-owned index.
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    slides: Vec<CarouselSlide>,
    index: usize,
    size: CarouselSize,
    theme: Option<CarouselTheme>,
    design: Option<Arc<DesignSystem>>,
    on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Carousel {
    /// Create a new empty carousel.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let carousel = Carousel::new("tour");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            slides: Vec::new(),
            index: 0,
            size: CarouselSize::default(),
            theme: None,
            design: None,
            on_change: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the full slide list.
    pub fn slides(mut self, slides: Vec<CarouselSlide>) -> Self {
        self.slides = slides;
        self
    }

    /// Append one slide to the rotation.
    pub fn slide(mut self, slide: CarouselSlide) -> Self {
        self.slides.push(slide);
        self
    }

    /// Set the active slide index (clamped at render).
    pub fn index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    /// Set the carousel size.
    pub fn size(mut self, size: CarouselSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: CarouselTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler called with the next index.
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Region).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the carousel with theme.
    pub fn build_with_theme(self, theme: &CarouselTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the carousel with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &CarouselTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let index = self.index.min(self.slides.len().saturating_sub(1));
        let root_id = self.id.clone();
        let mut root = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap));
        root = match self.size {
            CarouselSize::Sm => root.text_xs(),
            CarouselSize::Md => root.text_sm(),
            CarouselSize::Lg => root.text_lg(),
        };

        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap * 0.5))
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y))
            .rounded(px(design.corners.md))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.dot);
        match self.slides.get(index) {
            Some(slide) => {
                panel = panel
                    .child(div().text_color(theme.title).child(slide.title.clone()))
                    .child(div().text_color(theme.body).child(slide.body.clone()));
            }
            None => {
                panel = panel.child(div().text_color(theme.control).child("No slides"));
            }
        }
        root = root.child(panel);

        if self.slides.len() > 1 {
            let last = self.slides.len() - 1;
            let mut controls = div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(design.spacing.control_gap));
            controls = controls.child(Self::step_control(
                (root_id.clone(), "carousel-prev"),
                "‹",
                index.checked_sub(1),
                self.on_change.clone(),
            ));
            let mut dots = div()
                .flex()
                .items_center()
                .gap(px(design.spacing.control_gap * 0.5));
            for (dot_index, _) in self.slides.iter().enumerate() {
                let mut dot = div()
                    .id((
                        root_id.clone(),
                        SharedString::from(format!("carousel-dot-{dot_index}")),
                    ))
                    .w(px(design.spacing.grid_unit))
                    .h(px(design.spacing.grid_unit))
                    .rounded_full()
                    .bg(if dot_index == index {
                        theme.dot_active
                    } else {
                        theme.dot
                    });
                if dot_index != index
                    && let Some(ref on_change) = self.on_change
                {
                    let goto = on_change.clone();
                    dot = dot
                        .cursor_pointer()
                        .on_click(move |_event: &ClickEvent, window, cx| {
                            goto(dot_index, window, cx);
                        });
                }
                dots = dots.child(dot);
            }
            controls = controls.child(dots);
            controls = controls.child(Self::step_control(
                (root_id.clone(), "carousel-next"),
                "›",
                (index < last).then_some(index + 1),
                self.on_change.clone(),
            ));
            root = root.child(controls);
        }

        root
    }

    /// Previous/next control that reports `target` when enabled.
    fn step_control(
        id: impl Into<ElementId>,
        label: &'static str,
        target: Option<usize>,
        on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    ) -> Stateful<Div> {
        let mut control = div().id(id).child(label);
        match (target, on_change) {
            (Some(next), Some(handler)) => {
                let key_handler = handler.clone();
                control = control
                    .cursor_pointer()
                    .on_click(move |_event: &ClickEvent, window, cx| {
                        handler(next, window, cx);
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if key == "enter" || key == "space" {
                            key_handler(next, window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
            _ => {
                control = control.opacity(0.4);
            }
        }
        control
    }
}

impl RenderOnce for Carousel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| "Carousel".into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Region)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| CarouselTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Carousel, CarouselSize, CarouselSlide, CarouselTheme};
    use gpui::rgba;

    #[test]
    fn carousel_defaults_to_empty_standard_slides() {
        let carousel = Carousel::new("tour");
        assert!(carousel.slides.is_empty());
        assert_eq!(carousel.index, 0);
        assert_eq!(carousel.size, CarouselSize::Md);
        assert!(carousel.on_change.is_none());
    }

    #[test]
    fn carousel_builders_store_slides_index_and_size() {
        let carousel = Carousel::new("tour")
            .slide(CarouselSlide::new("Fast", "Starts fast."))
            .slides(vec![CarouselSlide::new("A", "First.")])
            .index(2)
            .size(CarouselSize::Lg);
        assert_eq!(carousel.slides.len(), 1);
        assert_eq!(carousel.slides[0].title.as_ref(), "A");
        assert_eq!(carousel.index, 2);
        assert_eq!(carousel.size, CarouselSize::Lg);
    }

    #[test]
    fn carousel_theme_defaults_match_compiled_tokens() {
        let theme = CarouselTheme::default();
        assert_eq!(theme.panel, rgba(0x1e1e1eff));
        assert_eq!(theme.title, rgba(0xffffffff));
        assert_eq!(theme.body, rgba(0xccccccff));
        assert_eq!(theme.dot, rgba(0x3e3e42ff));
        assert_eq!(theme.dot_active, rgba(0x007accff));
        assert_eq!(theme.control, rgba(0x777777ff));
    }
}
