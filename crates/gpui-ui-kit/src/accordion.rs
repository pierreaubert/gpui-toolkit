//! Accordion component
//!
//! Collapsible content sections with support for both vertical and horizontal orientations.

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, Hsla, MouseButton, SharedString, Stateful,
    TransformationMatrix, Window, canvas, div, px,
};

mod accordion_item;
mod misc;
mod types;

pub use accordion_item::AccordionItem;
use misc::rotated_side_label_svg;
use misc::side_label_height;
use types::AccordionChangeHandler;
pub use types::{AccordionMode, AccordionOrientation, AccordionTheme};

/// An accordion component
pub struct Accordion {
    items: Vec<AccordionItem>,
    expanded: Vec<SharedString>,
    mode: AccordionMode,
    orientation: AccordionOrientation,
    theme: Option<AccordionTheme>,
    on_change: Option<Box<dyn Fn(&SharedString, bool, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
    bordered: bool,
    rounded: bool,
    content_padding: bool,
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
            aria_label: None,
            aria_role: None,
            bordered: true,
            rounded: true,
            content_padding: true,
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

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Group)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Set whether the accordion draws its outer border.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set whether the accordion uses rounded outer corners.
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Set whether expanded content uses the default internal padding.
    pub fn content_padding(mut self, content_padding: bool) -> Self {
        self.content_padding = content_padding;
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

        if matches!(self.orientation, AccordionOrientation::Side) {
            let Accordion {
                items,
                expanded,
                on_change,
                bordered,
                rounded,
                content_padding,
                ..
            } = self;
            let on_change = on_change.map(|h| std::rc::Rc::new(h));
            return Self::build_side_layout_static(
                items,
                expanded,
                theme,
                on_change,
                bordered,
                rounded,
                content_padding,
            );
        }

        if matches!(self.orientation, AccordionOrientation::Horizontal) {
            let Accordion {
                items,
                expanded,
                on_change,
                bordered,
                rounded,
                content_padding,
                ..
            } = self;
            let on_change = on_change.map(|h| std::rc::Rc::new(h));
            return Self::build_horizontal_layout_static(
                items,
                expanded,
                theme,
                on_change,
                bordered,
                rounded,
                content_padding,
            );
        }

        let on_change = self.on_change.map(|h| std::rc::Rc::new(h));
        let mut container = div().flex().flex_col().w_full().min_w_0();
        if self.bordered {
            container = container.border_1().border_color(theme.border);
        }
        if self.rounded {
            container = container.rounded_lg();
        }

        for (idx, item) in self.items.into_iter().enumerate() {
            let is_expanded = self.expanded.contains(&item.id);
            let item_id = item.id.clone();
            let is_first = idx == 0;

            let header = Self::build_header_static(
                item_id,
                item.title,
                item.trailing,
                is_expanded,
                item.disabled,
                is_first,
                true,
                &theme,
                on_change.clone(),
            );
            let mut item_wrapper = div().w_full().child(header);

            // Content (only if expanded)
            if is_expanded && let Some(content) = item.content {
                let mut content_div =
                    div()
                        .bg(theme.content_bg)
                        .border_t_1()
                        .border_color(if is_expanded {
                            theme.accent_tint
                        } else {
                            theme.border
                        });
                if self.content_padding {
                    content_div = content_div.px_4().py_3();
                }

                item_wrapper = item_wrapper.child(content_div.child(content));
            }

            container = container.child(item_wrapper);
        }

        container
    }

    /// Build horizontal layout: tab headers on top, full-width content below
    fn build_horizontal_layout_static(
        items: Vec<AccordionItem>,
        expanded: Vec<SharedString>,
        theme: AccordionTheme,
        on_change: Option<AccordionChangeHandler>,
        bordered: bool,
        rounded: bool,
        content_padding: bool,
    ) -> Div {
        let mut container = div().flex().flex_col().w_full().min_w_0();
        if bordered {
            container = container.border_1().border_color(theme.border);
        }
        if rounded {
            container = container.rounded_lg();
        }

        let mut headers_container = div().flex().flex_row().items_stretch().w_full().min_w_0();
        let mut content_container = div().flex().flex_col().w_full();

        for (idx, item) in items.into_iter().enumerate() {
            let is_expanded = expanded.contains(&item.id);
            let item_id = item.id.clone();

            let header = Self::build_header_static(
                item_id,
                item.title,
                item.trailing,
                is_expanded,
                item.disabled,
                idx == 0,
                false,
                &theme,
                on_change.clone(),
            );
            headers_container = headers_container.child(header);

            if is_expanded && let Some(content) = item.content {
                let mut content_div = div()
                    .w_full()
                    .bg(theme.content_bg)
                    .border_t_1()
                    .border_color(theme.accent_tint);
                if content_padding {
                    content_div = content_div.px_4().py_3();
                }

                content_container = content_container.child(content_div.child(content));
            }
        }

        container = container.child(headers_container).child(content_container);

        container
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "accordion header builder carries independent render state for each item"
    )]
    fn build_header_static(
        item_id: SharedString,
        title: SharedString,
        trailing: Option<SharedString>,
        is_expanded: bool,
        disabled: bool,
        is_first: bool,
        is_vertical: bool,
        theme: &AccordionTheme,
        on_change: Option<AccordionChangeHandler>,
    ) -> Stateful<Div> {
        let mut header = div()
            .id(SharedString::from(format!("accordion-header-{}", item_id)))
            .flex()
            .gap_2()
            .w_full()
            .min_w_0()
            .px_4()
            .py_3()
            .bg(if is_expanded {
                theme.header_active_bg
            } else {
                theme.header_bg
            })
            .cursor_pointer();

        header = if is_vertical {
            header.items_start().flex_shrink_0()
        } else {
            header.items_start().flex_1().min_w_0()
        };

        if !is_first {
            header = if is_vertical {
                header.border_t_1().border_color(theme.border)
            } else {
                header.border_l_1().border_color(theme.border)
            };
        }

        if disabled {
            header = header.opacity(0.5).cursor_not_allowed();
        } else {
            let hover_bg = theme.header_hover_bg;
            let hover_accent = theme.accent_tint;
            header = header.hover(move |style| {
                style
                    .bg(hover_bg)
                    .border_color(hover_accent)
                    .shadow(glow_shadow(hover_bg))
            });

            if let Some(handler) = on_change {
                let id = item_id.clone();
                let new_state = !is_expanded;
                header = header.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    (handler)(&id, new_state, window, cx);
                });
            }
        }

        let rail_bg = if is_expanded {
            theme.accent
        } else {
            theme.accent_tint
        };
        header = header.child(
            div()
                .w(px(3.0))
                .h(px(22.0))
                .flex_shrink_0()
                .rounded(px(1.5))
                .bg(rail_bg),
        );

        header = header.child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .line_height(px(20.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.title_color)
                .child(title),
        );

        if let Some(trailing) = trailing {
            header = header.child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .line_height(px(20.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.indicator_color)
                    .whitespace_nowrap()
                    .child(trailing),
            );
        }

        let indicator = if is_vertical {
            if is_expanded { "▼" } else { "▶" }
        } else if is_expanded {
            "▼"
        } else {
            "▲"
        };
        header.child(
            div()
                .text_xs()
                .line_height(px(20.0))
                .flex_shrink_0()
                .text_color(if is_expanded {
                    theme.accent
                } else {
                    theme.indicator_color
                })
                .child(indicator),
        )
    }

    /// Build side layout: vertical tab bars split around the active content
    fn build_side_layout_static(
        items: Vec<AccordionItem>,
        expanded: Vec<SharedString>,
        theme: AccordionTheme,
        on_change: Option<AccordionChangeHandler>,
        bordered: bool,
        rounded: bool,
        content_padding: bool,
    ) -> Div {
        let active_index = items
            .iter()
            .position(|item| expanded.contains(&item.id))
            .unwrap_or(usize::MAX);
        let mut container = div().flex().flex_row().min_h(px(120.0));
        if bordered {
            container = container.border_1().border_color(theme.border);
        }
        if rounded {
            container = container.rounded_lg();
        }

        let mut left_tabs = div().flex().flex_row().h_full();
        let mut right_tabs = div().flex().flex_row().h_full();
        let mut content_container = div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .bg(theme.content_bg);
        if bordered {
            content_container = content_container
                .border_x_1()
                .border_color(theme.accent_tint);
        }
        let mut left_count = 0;
        let mut right_count = 0;

        for (idx, item) in items.into_iter().enumerate() {
            let is_expanded = expanded.contains(&item.id);
            let item_id = item.id.clone();
            let goes_left = active_index == usize::MAX || idx <= active_index;
            let is_first_in_group = if goes_left {
                left_count == 0
            } else {
                right_count == 0
            };

            let tab = Self::build_side_tab_static(
                item_id,
                item.title,
                is_expanded,
                item.disabled,
                is_first_in_group,
                !goes_left,
                &theme,
                on_change.clone(),
            );

            if goes_left {
                left_tabs = left_tabs.child(tab);
                left_count += 1;
            } else {
                right_tabs = right_tabs.child(tab);
                right_count += 1;
            }

            if is_expanded && let Some(content) = item.content {
                let mut content_div = div().w_full().bg(theme.content_bg);
                if content_padding {
                    content_div = content_div.px_4().py_3();
                }

                content_container = content_container.child(content_div.child(content));
            }
        }

        container = container
            .child(left_tabs)
            .child(content_container)
            .child(right_tabs);

        container
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "side-tab builder carries independent render state for each item"
    )]
    fn build_side_tab_static(
        item_id: SharedString,
        title: SharedString,
        is_expanded: bool,
        disabled: bool,
        is_first_in_group: bool,
        rail_on_right: bool,
        theme: &AccordionTheme,
        on_change: Option<AccordionChangeHandler>,
    ) -> Stateful<Div> {
        let mut header = div()
            .id(SharedString::from(format!(
                "accordion-header-side-{}",
                item_id
            )))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .w(px(42.0))
            .h_full()
            .min_h(px(120.0))
            .bg(if is_expanded {
                theme.header_active_bg
            } else {
                theme.header_bg
            })
            .cursor_pointer();

        if !is_first_in_group {
            header = header.border_l_1().border_color(theme.border);
        }

        if disabled {
            header = header.opacity(0.5).cursor_not_allowed();
        } else {
            let hover_bg = theme.header_hover_bg;
            let hover_accent = theme.accent_tint;
            header = header.hover(move |style| {
                style
                    .bg(hover_bg)
                    .border_color(hover_accent)
                    .shadow(glow_shadow(hover_bg))
            });

            if let Some(handler) = on_change {
                let id = item_id.clone();
                let new_state = !is_expanded;
                header = header.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    (handler)(&id, new_state, window, cx);
                });
            }
        }

        let label_text = title.to_string();
        let label_height = side_label_height(&title);
        let label_svg = rotated_side_label_svg(&label_text);
        let label_path = SharedString::from(format!("accordion-side-label:{item_id}:{label_text}"));
        let label_color = if is_expanded {
            theme.accent
        } else {
            theme.title_color
        };

        let mut rail = div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(px(3.0))
            .bg(if is_expanded {
                theme.accent
            } else {
                theme.accent_tint
            });
        rail = if rail_on_right {
            rail.right_0()
        } else {
            rail.left_0()
        };

        header.child(rail).child(
            canvas(
                move |_bounds, _window, _cx| label_svg,
                move |bounds, label_svg, window, cx| {
                    let _ = window.paint_svg(
                        bounds,
                        label_path,
                        Some(label_svg.as_bytes()),
                        TransformationMatrix::unit(),
                        Hsla::from(label_color),
                        cx,
                    );
                },
            )
            .w(px(18.0))
            .h(label_height),
        )
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        cx.register_accessible(AccessibilityNode {
            element_id: ElementId::Name("accordion".into()),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let accordion_theme = AccordionTheme::from(global_theme);
        self.build_with_theme(&accordion_theme)
    }
}

impl IntoElement for Accordion {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
