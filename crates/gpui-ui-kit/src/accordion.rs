//! Accordion component
//!
//! Collapsible content sections with support for both vertical and horizontal orientations.

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Theme colors for accordion styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct AccordionTheme {
    #[theme(default = 0x252525, from = muted)]
    pub header_bg: Rgba,
    #[theme(default = 0x2a2a2a, from = surface_hover)]
    pub header_hover_bg: Rgba,
    #[theme(default = 0x1e1e1e, from = background)]
    pub content_bg: Rgba,
    #[theme(default = 0x3a3a3a, from = border)]
    pub border: Rgba,
    #[theme(default = 0xffffff, from = text_primary)]
    pub title_color: Rgba,
    #[theme(default = 0x888888, from = text_muted)]
    pub indicator_color: Rgba,
}

/// A single accordion item
pub struct AccordionItem {
    id: SharedString,
    title: SharedString,
    content: Option<AnyElement>,
    disabled: bool,
}

impl AccordionItem {
    /// Create a new accordion item
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content: None,
            disabled: false,
        }
    }

    /// Set content
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Get the item ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }
}

/// Accordion behavior mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccordionMode {
    /// Only one item can be open at a time
    #[default]
    Single,
    /// Multiple items can be open
    Multiple,
}

/// Accordion orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccordionOrientation {
    /// Vertical layout: headers stacked vertically, content expands downward (default)
    #[default]
    Vertical,
    /// Horizontal layout: headers arranged horizontally, content expands downward
    Horizontal,
    /// Side layout: headers stacked vertically on left, content expands to right
    Side,
}

/// An accordion component
pub struct Accordion {
    items: Vec<AccordionItem>,
    expanded: Vec<SharedString>,
    mode: AccordionMode,
    orientation: AccordionOrientation,
    theme: Option<AccordionTheme>,
    on_change: Option<Box<dyn Fn(&SharedString, bool, &mut Window, &mut App) + 'static>>,
}

impl Accordion {
    /// Create a new accordion
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            expanded: Vec::new(),
            mode: AccordionMode::default(),
            orientation: AccordionOrientation::default(),
            theme: None,
            on_change: None,
        }
    }

    /// Set items
    pub fn items(mut self, items: Vec<AccordionItem>) -> Self {
        self.items = items;
        self
    }

    /// Add a single item
    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set expanded item IDs
    pub fn expanded(mut self, expanded: Vec<SharedString>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set mode
    pub fn mode(mut self, mode: AccordionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: AccordionOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: AccordionTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set change handler (receives item ID and new expanded state)
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &AccordionTheme) -> Div {
        // Use self.theme if provided, otherwise clone the passed theme
        let theme = self.theme.unwrap_or_else(|| theme.clone());

        // Handle Side layout separately since it needs different structure
        let is_side = matches!(self.orientation, AccordionOrientation::Side);
        if is_side {
            let Accordion {
                items,
                expanded,
                on_change,
                ..
            } = self;
            let on_change = on_change.map(|h| std::rc::Rc::new(h));
            return Self::build_side_layout_static(items, expanded, theme, on_change);
        }

        let on_change = self.on_change.map(|h| std::rc::Rc::new(h));
        let is_vertical = matches!(self.orientation, AccordionOrientation::Vertical);

        let mut container = div()
            .flex()
            .border_1()
            .border_color(theme.border)
            .rounded_lg();

        // Set flex direction based on orientation
        container = if is_vertical {
            container.flex_col()
        } else {
            container.flex_row()
        };

        for (idx, item) in self.items.into_iter().enumerate() {
            let is_expanded = self.expanded.contains(&item.id);
            let item_id = item.id.clone();
            let is_first = idx == 0;

            // Create item wrapper for horizontal layout
            let mut item_wrapper = div();
            if !is_vertical {
                item_wrapper = item_wrapper.flex().flex_col();
            }

            // Header
            let mut header = div()
                .id(SharedString::from(format!("accordion-header-{}", item_id)))
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .bg(theme.header_bg)
                .cursor_pointer();

            // Add border based on orientation
            if !is_first {
                header = if is_vertical {
                    header.border_t_1().border_color(theme.border)
                } else {
                    header.border_l_1().border_color(theme.border)
                };
            }

            if item.disabled {
                header = header.opacity(0.5).cursor_not_allowed();
            } else {
                let hover_bg = theme.header_hover_bg;
                header =
                    header.hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));

                // Click handler
                if let Some(handler) = on_change.clone() {
                    let id = item_id.clone();
                    let new_state = !is_expanded;
                    header = header.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                        (handler)(&id, new_state, window, cx);
                    });
                }
            }

            // Title
            header = header.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.title_color)
                    .child(item.title),
            );

            // Expand/collapse indicator (different for vertical vs horizontal)
            let indicator = if is_vertical {
                if is_expanded { "▼" } else { "▶" }
            } else if is_expanded {
                "▼"
            } else {
                "▲"
            };
            header = header.child(
                div()
                    .text_xs()
                    .text_color(theme.indicator_color)
                    .child(indicator),
            );

            item_wrapper = item_wrapper.child(header);

            // Content (only if expanded)
            if is_expanded && let Some(content) = item.content {
                let content_div = div()
                    .px_4()
                    .py_3()
                    .bg(theme.content_bg)
                    .border_t_1()
                    .border_color(theme.border);

                item_wrapper = item_wrapper.child(content_div.child(content));
            }

            container = container.child(item_wrapper);
        }

        container
    }

    /// Build side layout: headers vertically on left, content expands to right
    fn build_side_layout_static(
        items: Vec<AccordionItem>,
        expanded: Vec<SharedString>,
        theme: AccordionTheme,
        on_change: Option<
            std::rc::Rc<Box<dyn Fn(&SharedString, bool, &mut Window, &mut App) + 'static>>,
        >,
    ) -> Div {
        let mut container = div()
            .flex()
            .flex_row()
            .border_1()
            .border_color(theme.border)
            .rounded_lg();

        // Left side: vertical header tabs
        let mut headers_container = div()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(theme.border);

        for (idx, item) in items.iter().enumerate() {
            let is_expanded = expanded.contains(&item.id);
            let item_id = item.id.clone();
            let is_first = idx == 0;

            let mut header = div()
                .id(SharedString::from(format!(
                    "accordion-header-side-{}",
                    item_id
                )))
                .flex()
                .items_center()
                .justify_center()
                .w(px(40.0))
                .py_4()
                .bg(theme.header_bg)
                .cursor_pointer();

            if !is_first {
                header = header.border_t_1().border_color(theme.border);
            }

            if is_expanded {
                header = header.bg(theme.header_hover_bg);
            }

            if item.disabled {
                header = header.opacity(0.5).cursor_not_allowed();
            } else {
                let hover_bg = theme.header_hover_bg;
                header =
                    header.hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));

                // Click handler
                if let Some(handler) = on_change.clone() {
                    let id = item_id.clone();
                    let new_state = !is_expanded;
                    header = header.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                        (handler)(&id, new_state, window, cx);
                    });
                }
            }

            // Vertical text - display each character on its own line
            let mut text_container = div().flex().flex_col().items_center().gap_1();

            // Show full text vertically when expanded, abbreviated when closed
            if is_expanded {
                // Show full text vertically
                for ch in item.title.chars() {
                    text_container = text_container.child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.title_color)
                            .child(ch.to_string()),
                    );
                }
            } else {
                // Show first character only when closed
                let label_text = if !item.title.is_empty() {
                    item.title.chars().next().unwrap().to_string()
                } else {
                    String::from("?")
                };
                text_container = text_container.child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.title_color)
                        .child(label_text),
                );
            }

            header = header.child(text_container);

            headers_container = headers_container.child(header);
        }

        container = container.child(headers_container);

        // Right side: content area - show all expanded items side by side
        let mut content_container = div().flex().flex_row().flex_1();

        for item in items.into_iter() {
            let is_expanded = expanded.contains(&item.id);

            if is_expanded && let Some(content) = item.content {
                let content_div = div()
                    .flex_1()
                    .px_4()
                    .py_3()
                    .bg(theme.content_bg)
                    .border_r_1()
                    .border_color(theme.border)
                    .child(content);

                content_container = content_container.child(content_div);
            }
        }

        container = container.child(content_container);

        container
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let accordion_theme = AccordionTheme::from(&global_theme);
        self.build_with_theme(&accordion_theme)
    }
}

impl IntoElement for Accordion {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
