//! Per-variant tab builders extracted from `TabsEntity::render`.
//!
//! Each helper takes explicit parameters (no shared render locals) so the
//! main render loop stays a thin dispatch over [`TabVariant`].

use super::TabsEntity;
use super::tab_item::TabItem;
use super::types::{TabVariant, TabsTheme};
use crate::theme::glow_shadow;
use gpui::prelude::{
    FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
};
use gpui::{
    Context, Div, ElementId, FontWeight, MouseButton, MouseDownEvent, Rgba, SharedString, Stateful,
    div, px,
};

/// Copyable theme colors consumed by the tab builders.
#[derive(Debug, Clone, Copy)]
pub(super) struct TabColors {
    pub text_hover: Rgba,
    pub hover_bg: Rgba,
    pub selected_bg: Rgba,
    pub close_hover_color: Rgba,
    pub text_selected: Rgba,
    pub text_unselected: Rgba,
    pub accent: Rgba,
    pub container_bg: Rgba,
    pub container_border: Rgba,
    pub badge_bg: Rgba,
    pub close_color: Rgba,
    pub icon_selected: Option<Rgba>,
    pub icon_unselected: Option<Rgba>,
}

impl TabColors {
    pub(super) fn from_theme(theme: &TabsTheme) -> Self {
        Self {
            text_hover: theme.text_hover,
            hover_bg: theme.hover_bg,
            selected_bg: theme.selected_bg,
            close_hover_color: theme.close_hover_color,
            text_selected: theme.text_selected,
            text_unselected: theme.text_unselected,
            accent: theme.accent,
            container_bg: theme.container_bg,
            container_border: theme.container_border,
            badge_bg: theme.badge_bg,
            close_color: theme.close_color,
            icon_selected: theme.icon_selected,
            icon_unselected: theme.icon_unselected,
        }
    }
}

/// Per-tab render inputs shared by every variant builder.
pub(super) struct TabRenderState {
    pub index: usize,
    pub is_selected: bool,
    pub hovered: bool,
    pub close_hovered: bool,
}

/// Variant-specific container styling for the tab bar itself.
pub(super) fn style_tab_container(
    mut container: Stateful<Div>,
    variant: TabVariant,
    colors: &TabColors,
) -> Stateful<Div> {
    match variant {
        TabVariant::Underline => {
            // No border on container - we'll add underlines per-tab
        }
        TabVariant::Enclosed => {
            container = container.gap_1();
        }
        TabVariant::Pills => {
            container = container.gap_2().p_1().bg(colors.container_bg).rounded_lg();
        }
        TabVariant::VerticalCard => {
            container = container
                .flex_wrap()
                .gap_2()
                .p_1()
                .bg(colors.container_bg)
                .rounded_lg();
        }
    }
    container
}

/// Small badge pill shared by the underline and regular variants.
pub(super) fn render_badge(badge: SharedString, colors: &TabColors) -> Div {
    div()
        .text_xs()
        .px_1()
        .py(px(1.0))
        .bg(colors.badge_bg)
        .rounded(px(3.0))
        .child(badge)
}

/// Close button shared by the underline and regular variants.
pub(super) fn render_close_button(
    close_element_id: ElementId,
    index: usize,
    close_hovered: bool,
    colors: &TabColors,
    cx: &mut Context<TabsEntity>,
) -> Stateful<Div> {
    let mut close_btn = div()
        .id(close_element_id)
        .text_xs()
        .text_color(colors.close_color)
        .when(close_hovered, |s| s.text_color(colors.close_hover_color));

    close_btn = close_btn
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                this.handle_close_click(index, event, window, cx);
            }),
        )
        .on_hover(
            cx.listener(move |this: &mut TabsEntity, hovered: &bool, _window, cx| {
                this.set_close_hovered(index, *hovered, cx);
            }),
        );

    close_btn.child("×")
}

/// Hover bookkeeping shared by every tab: row hover plus close hover.
pub(super) fn with_tab_hover(
    tab_element: Stateful<Div>,
    index: usize,
    disabled: bool,
    cx: &mut Context<TabsEntity>,
) -> Stateful<Div> {
    if disabled {
        tab_element
    } else {
        tab_element.on_hover(cx.listener(
            move |this: &mut TabsEntity, hovered: &bool, _window, cx| {
                this.set_hovered(index, *hovered, cx);
            },
        ))
    }
}

/// Underline variant: content row plus a 2px/1px indicator underneath.
pub(super) fn render_underline_tab(
    tab: TabItem,
    state: &TabRenderState,
    colors: &TabColors,
    cx: &mut Context<TabsEntity>,
) -> Stateful<Div> {
    let TabRenderState {
        index,
        is_selected,
        hovered,
        close_hovered,
    } = *state;
    let TabItem {
        label,
        icon,
        custom_icon,
        badge,
        disabled,
        closeable,
        tab_element_id,
        close_element_id,
        wrapper_element_id,
        ..
    } = tab;

    let mut tab_content = div()
        .id(tab_element_id)
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2();

    if is_selected {
        tab_content = tab_content
            .text_color(colors.text_selected)
            .font_weight(FontWeight::SEMIBOLD);
    } else {
        tab_content = tab_content
            .text_color(colors.text_unselected)
            .when(hovered, |s| s.text_color(colors.text_hover));
    }

    if disabled {
        tab_content = tab_content.opacity(0.5).cursor_not_allowed();
    } else {
        tab_content = tab_content.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                this.handle_tab_click(index, event, window, cx);
            }),
        );
    }

    // Add icon
    if let Some(custom_icon) = custom_icon {
        tab_content = tab_content.child(custom_icon);
    } else if let Some(icon) = icon {
        tab_content = tab_content.child(div().text_sm().child(icon));
    }

    // Add label
    tab_content = tab_content.child(div().text_sm().child(label));

    // Add badge
    if let Some(badge) = badge {
        tab_content = tab_content.child(render_badge(badge, colors));
    }

    // Add close button
    if closeable {
        let close_btn = render_close_button(close_element_id, index, close_hovered, colors, cx);
        tab_content = tab_content.child(close_btn);
    }

    let tab_content = with_tab_hover(tab_content, index, disabled, cx);

    // Create the underline - accent color for selected, border color for unselected
    let underline = if is_selected {
        div().h(px(2.0)).w_full().bg(colors.accent)
    } else {
        div().h(px(1.0)).w_full().bg(colors.container_border)
    };

    // Wrap in a flex column
    div()
        .id(wrapper_element_id)
        .flex()
        .flex_col()
        .child(tab_content)
        .child(underline)
}

/// VerticalCard variant: icon on the left, title plus badge on the right.
pub(super) fn render_card_tab(
    tab: TabItem,
    state: &TabRenderState,
    colors: &TabColors,
    cx: &mut Context<TabsEntity>,
) -> Stateful<Div> {
    let TabRenderState {
        index,
        is_selected,
        hovered,
        ..
    } = *state;
    let TabItem {
        label,
        icon,
        custom_icon,
        icon_factory,
        badge,
        disabled,
        tab_element_id,
        ..
    } = tab;

    let mut tab_el = div()
        .id(tab_element_id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .min_w(px(90.0));

    if is_selected {
        tab_el = tab_el
            .bg(colors.accent)
            .rounded_lg()
            .text_color(colors.text_selected);
    } else {
        tab_el = tab_el
            .bg(colors.selected_bg)
            .rounded_lg()
            .text_color(colors.text_unselected)
            .when(hovered, |s| {
                s.bg(colors.selected_bg)
                    .text_color(colors.text_hover)
                    .shadow(glow_shadow(colors.selected_bg))
            });
    }

    if disabled {
        tab_el = tab_el.opacity(0.5).cursor_not_allowed();
    } else {
        tab_el = tab_el.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                this.handle_tab_click(index, event, window, cx);
            }),
        );
    }

    // Icon on left (large, spans both rows visually)
    let icon_color = if is_selected {
        colors.icon_selected.unwrap_or(colors.text_selected)
    } else {
        colors.icon_unselected.unwrap_or(colors.accent)
    };
    if let Some(factory) = icon_factory {
        let icon_element = factory(icon_color);
        tab_el = tab_el.child(div().flex().items_center().child(icon_element));
    } else if let Some(custom_icon) = custom_icon {
        tab_el = tab_el.child(
            div()
                .flex()
                .items_center()
                .text_color(icon_color)
                .child(custom_icon),
        );
    } else if let Some(icon) = icon {
        tab_el = tab_el.child(
            div()
                .flex()
                .items_center()
                .text_xl()
                .text_color(icon_color)
                .child(icon),
        );
    }

    // Right side: Title on top, Number below
    let mut right_col = div().flex().flex_col().gap(px(1.0));

    right_col = right_col.child(
        div()
            .text_xs()
            .font_weight(if is_selected {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .child(label),
    );

    if let Some(badge) = badge {
        right_col = right_col.child(div().text_sm().font_weight(FontWeight::BOLD).child(badge));
    }

    tab_el = tab_el.child(right_col);

    with_tab_hover(tab_el, index, disabled, cx)
}

/// Enclosed/Pills variants: icon, label, badge, and close button in one row.
pub(super) fn render_regular_tab(
    tab: TabItem,
    variant: TabVariant,
    state: &TabRenderState,
    colors: &TabColors,
    cx: &mut Context<TabsEntity>,
) -> Stateful<Div> {
    let TabRenderState {
        index,
        is_selected,
        hovered,
        close_hovered,
    } = *state;
    let TabItem {
        label,
        icon,
        custom_icon,
        badge,
        disabled,
        closeable,
        tab_element_id,
        close_element_id,
        ..
    } = tab;

    let mut tab_el = div()
        .id(tab_element_id)
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2();

    match variant {
        TabVariant::Enclosed => {
            if is_selected {
                tab_el = tab_el
                    .bg(colors.selected_bg)
                    .rounded_t_md()
                    .text_color(colors.text_selected);
            } else {
                tab_el = tab_el
                    .text_color(colors.text_unselected)
                    .when(hovered, |s| {
                        s.bg(colors.hover_bg)
                            .text_color(colors.text_hover)
                            .shadow(glow_shadow(colors.hover_bg))
                    });
            }
        }
        TabVariant::Pills => {
            if is_selected {
                tab_el = tab_el
                    .bg(colors.accent)
                    .rounded_md()
                    .text_color(colors.text_selected);
            } else {
                tab_el =
                    tab_el
                        .rounded_md()
                        .text_color(colors.text_unselected)
                        .when(hovered, |s| {
                            s.bg(colors.selected_bg)
                                .text_color(colors.text_hover)
                                .shadow(glow_shadow(colors.selected_bg))
                        });
            }
        }
        TabVariant::Underline | TabVariant::VerticalCard => unreachable!(),
    }

    if disabled {
        tab_el = tab_el.opacity(0.5).cursor_not_allowed();
    } else {
        tab_el = tab_el.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                this.handle_tab_click(index, event, window, cx);
            }),
        );
    }

    // Add icon
    if let Some(custom_icon) = custom_icon {
        tab_el = tab_el.child(custom_icon);
    } else if let Some(icon) = icon {
        tab_el = tab_el.child(div().text_sm().child(icon));
    }

    // Add label
    tab_el = tab_el.child(div().text_sm().child(label));

    // Add badge
    if let Some(badge) = badge {
        tab_el = tab_el.child(render_badge(badge, colors));
    }

    // Add close button
    if closeable {
        let close_btn = render_close_button(close_element_id, index, close_hovered, colors, cx);
        tab_el = tab_el.child(close_btn);
    }

    with_tab_hover(tab_el, index, disabled, cx)
}
