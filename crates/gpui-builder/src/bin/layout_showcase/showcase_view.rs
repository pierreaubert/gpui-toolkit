use super::drag_session::DragSession;
use super::misc::INSPECTOR_TIERS;
use super::misc::axis_text;
use super::misc::muted;
use super::misc::panel_box;
use super::misc::size_label;
use super::showcase_theme::ShowcaseTheme;
use super::types::DragTarget;
use super::types::VisualTreeRow;
use super::types::collect_visual_tree_rows;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point, Render, Rgba, Role, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
    div, px, rgba,
};
use gpui_builder::types::LayoutPreferences;
use gpui_builder::{
    Axis, ContainerNode, LayoutNode, RetainedLayoutSolver, Sizing, SlotNode, SolvedTree,
};
use gpui_design::DesignExt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn build_showcase_layout_root() -> &'static LayoutNode<'static> {
    let content_children = Box::leak(Box::new([
        LayoutNode::Slot(SlotNode {
            id: "sidebar",
            sizing: Sizing::fractional(0.22, 80.0),
            priority: 0.5,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Sidebar"),
        }),
        LayoutNode::Slot(SlotNode {
            id: "main",
            sizing: Sizing::flex(200.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Slot(SlotNode {
            id: "inspector",
            sizing: Sizing::fractional(0.25, 0.0),
            priority: 0.3,
            collapsible: true,
            display_tiers: INSPECTOR_TIERS,
            collapse_label: Some("Inspector"),
        }),
    ]));
    let root_children = Box::leak(Box::new([
        LayoutNode::Slot(SlotNode {
            id: "header",
            sizing: Sizing::Fixed(44.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Container(ContainerNode {
            id: "content",
            axis: Axis::Horizontal,
            auto_axis: Some(1.0),
            sizing: Sizing::flex(0.0),
            children: content_children,
            divider_size: 6.0,
        }),
        LayoutNode::Slot(SlotNode {
            id: "footer",
            sizing: Sizing::Fixed(32.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
    ]));
    Box::leak(Box::new(LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: root_children,
        divider_size: 0.0,
    })))
}

thread_local! {
    static SHOWCASE_LAYOUT_ROOT: &'static LayoutNode<'static> = build_showcase_layout_root();
}

fn showcase_layout_root() -> &'static LayoutNode<'static> {
    SHOWCASE_LAYOUT_ROOT.with(|root| *root)
}

static DEFAULT_RATIOS: [(&str, Axis, f32); 4] = [
    ("sidebar", Axis::Horizontal, 0.22),
    ("sidebar", Axis::Vertical, 0.25),
    ("inspector", Axis::Horizontal, 0.25),
    ("inspector", Axis::Vertical, 0.20),
];

static DEFAULT_COLLAPSED: [(&str, bool); 2] = [("sidebar", false), ("inspector", false)];

pub(super) struct ShowcaseView {
    pub(super) sidebar_ratio_h: f32,
    pub(super) sidebar_ratio_v: f32,
    pub(super) inspector_ratio_h: f32,
    pub(super) inspector_ratio_v: f32,
    pub(super) sidebar_collapsed: bool,
    pub(super) inspector_collapsed: bool,
    pub(super) dragging: Option<DragSession>,
    pub(super) drag_moved: bool,
    pub(super) selected_node: Option<String>,
    pub(super) render_probe: Option<Arc<AtomicUsize>>,
    layout_solver: RetainedLayoutSolver<'static>,
    layout_preferences: LayoutPreferences<'static>,
    layout_root: &'static LayoutNode<'static>,
}

impl ShowcaseView {
    pub(super) fn new(render_probe: Option<Arc<AtomicUsize>>) -> Self {
        let layout_root = showcase_layout_root();
        Self {
            sidebar_ratio_h: 0.22,
            sidebar_ratio_v: 0.25,
            inspector_ratio_h: 0.25,
            inspector_ratio_v: 0.20,
            sidebar_collapsed: false,
            inspector_collapsed: false,
            dragging: None,
            drag_moved: false,
            selected_node: Some("root".to_string()),
            render_probe,
            layout_solver: RetainedLayoutSolver::with_capacity(layout_root.node_count()),
            layout_preferences: LayoutPreferences::new(&DEFAULT_RATIOS, &DEFAULT_COLLAPSED),
            layout_root,
        }
    }

    pub(super) fn begin_drag(
        &mut self,
        target: DragTarget,
        axis: Axis,
        start_pos: f32,
        extent: f32,
    ) {
        let start_ratio = match (target, axis) {
            (DragTarget::Sidebar, Axis::Horizontal) => self.sidebar_ratio_h,
            (DragTarget::Sidebar, Axis::Vertical) => self.sidebar_ratio_v,
            (DragTarget::Inspector, Axis::Horizontal) => self.inspector_ratio_h,
            (DragTarget::Inspector, Axis::Vertical) => self.inspector_ratio_v,
        };

        self.dragging = Some(DragSession {
            target,
            axis,
            start_pos,
            start_ratio,
            extent: extent.max(1.0),
        });
        self.drag_moved = false;
    }

    pub(super) fn update_drag_from_position(&mut self, position: Point<Pixels>) -> bool {
        let Some(drag) = self.dragging else {
            return false;
        };
        let delta = (drag.axis_position(position) - drag.start_pos) / drag.extent;
        let next = match drag.target {
            DragTarget::Sidebar => drag.start_ratio + delta,
            DragTarget::Inspector => drag.start_ratio - delta,
        }
        .clamp(0.08, 0.45);

        let ratio = match (drag.target, drag.axis) {
            (DragTarget::Sidebar, Axis::Horizontal) => &mut self.sidebar_ratio_h,
            (DragTarget::Sidebar, Axis::Vertical) => &mut self.sidebar_ratio_v,
            (DragTarget::Inspector, Axis::Horizontal) => &mut self.inspector_ratio_h,
            (DragTarget::Inspector, Axis::Vertical) => &mut self.inspector_ratio_v,
        };

        if (*ratio - next).abs() <= 0.001 {
            return false;
        }

        *ratio = next;
        self.sync_layout_preferences();
        self.drag_moved = true;
        true
    }

    /// Applies physical divider movement. A right/bottom inspector divider
    /// moves toward the inspector, so its ratio changes in the opposite
    /// direction from the leading sidebar divider.
    fn nudge_ratio(&mut self, target: DragTarget, axis: Axis, movement: f32) -> bool {
        let ratio = match (target, axis) {
            (DragTarget::Sidebar, Axis::Horizontal) => &mut self.sidebar_ratio_h,
            (DragTarget::Sidebar, Axis::Vertical) => &mut self.sidebar_ratio_v,
            (DragTarget::Inspector, Axis::Horizontal) => &mut self.inspector_ratio_h,
            (DragTarget::Inspector, Axis::Vertical) => &mut self.inspector_ratio_v,
        };
        let delta = match target {
            DragTarget::Sidebar => movement,
            DragTarget::Inspector => -movement,
        };
        let next = (*ratio + delta).clamp(0.08, 0.45);
        if (*ratio - next).abs() <= 0.001 {
            return false;
        }

        *ratio = next;
        self.sync_layout_preferences();
        true
    }

    fn sync_layout_preferences(&mut self) {
        self.layout_preferences
            .set_ratio("sidebar", Axis::Horizontal, self.sidebar_ratio_h);
        self.layout_preferences
            .set_ratio("sidebar", Axis::Vertical, self.sidebar_ratio_v);
        self.layout_preferences
            .set_ratio("inspector", Axis::Horizontal, self.inspector_ratio_h);
        self.layout_preferences
            .set_ratio("inspector", Axis::Vertical, self.inspector_ratio_v);
        self.layout_preferences
            .set_collapsed("sidebar", self.sidebar_collapsed);
        self.layout_preferences
            .set_collapsed("inspector", self.inspector_collapsed);
    }

    pub(super) fn finish_drag(&mut self, position: Point<Pixels>) -> bool {
        let Some(drag) = self.dragging.take() else {
            return false;
        };
        let moved = self.drag_moved || (drag.axis_position(position) - drag.start_pos).abs() > 3.0;
        if !moved {
            match drag.target {
                DragTarget::Sidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
                DragTarget::Inspector => self.inspector_collapsed = !self.inspector_collapsed,
            }
        }
        self.sync_layout_preferences();
        self.drag_moved = false;
        true
    }
}

impl Render for ShowcaseView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(render_probe) = &self.render_probe {
            let render_index = render_probe.fetch_add(1, Ordering::Release);
            if render_index == 0 {
                // Defer the mutation until the initial paint returns. A
                // render-time invalidation is dropped by Xvfb's event loop.
                cx.defer_in(window, |view, _window, cx| {
                    view.sidebar_collapsed = true;
                    view.sync_layout_preferences();
                    cx.notify();
                });
            }
        }
        let theme = ShowcaseTheme::from_window_appearance(window.appearance());
        let ds = cx.design();
        let bounds = window.bounds();
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();

        let solved = self
            .layout_solver
            .solve(self.layout_root, w, h, &self.layout_preferences);

        let selected_id = self.selected_node.as_deref().unwrap_or("root");

        let content = solved.find("content").unwrap();
        let is_h = content.resolved_axis() == Some(Axis::Horizontal);
        let header_h = solved.find("header").unwrap().height();
        let footer_h = solved.find("footer").unwrap().height();
        let mut tab_storage = [("", ""); 2];
        let mut tab_count = 0;
        for slot in solved.collapsed_slots() {
            debug_assert!(tab_count < tab_storage.len());
            if let Some(tab) = tab_storage.get_mut(tab_count) {
                *tab = (slot.id, slot.label);
                tab_count += 1;
            }
        }
        let tabs = &tab_storage[..tab_count];

        let sidebar = solved.find("sidebar").unwrap();
        let main_n = solved.find("main").unwrap();
        let inspector = solved.find("inspector").unwrap();
        let content_w = content.width();
        let content_h = content.height();

        // Colors — tinted variants of surface to distinguish panels
        let header_bg = theme.surface;
        let footer_bg = theme.surface;
        let sidebar_bg = theme.muted;
        let main_bg = theme.background;
        let inspector_bg = theme.muted;
        let divider_color = theme.border;
        let accent = theme.accent;
        let fg = theme.text_primary;
        let base_sz = ds.typography.base_size;
        let small_sz = ds.typography.small_size;

        let axis_label = if is_h { "Horizontal" } else { "Vertical" };
        let sidebar_pct = if is_h {
            self.sidebar_ratio_h
        } else {
            self.sidebar_ratio_v
        } * 100.0;
        let inspector_pct = if is_h {
            self.inspector_ratio_h
        } else {
            self.inspector_ratio_v
        } * 100.0;

        div()
            .id("showcase-root")
            .debug_selector(|| "showcase-root".to_string())
            .size_full()
            .bg(theme.background)
            .text_color(fg)
            .flex()
            .flex_col()
            .when(selected_id == "root", |d| {
                d.border_1().border_color(muted(accent, 0.8))
            })
            .on_mouse_move(
                cx.listener(move |view, event: &MouseMoveEvent, _window, cx| {
                    if view.update_drag_from_position(event.position) {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, event: &MouseUpEvent, _, cx| {
                    let changed = view.update_drag_from_position(event.position);
                    let finished = view.finish_drag(event.position);
                    if changed || finished {
                        cx.notify();
                    }
                }),
            )
            // ---- Header ----
            .child(Self::render_header(
                header_h,
                header_bg,
                selected_id,
                accent,
                &theme,
                &ds,
                base_sz,
                small_sz,
                w,
                h,
                axis_label,
            ))
            // ---- Content ----
            .child(Self::render_content(
                &solved,
                selected_id,
                is_h,
                if is_h {
                    self.sidebar_ratio_h
                } else {
                    self.sidebar_ratio_v
                },
                if is_h {
                    self.inspector_ratio_h
                } else {
                    self.inspector_ratio_v
                },
                content_w,
                content_h,
                sidebar,
                main_n,
                inspector,
                sidebar_bg,
                main_bg,
                inspector_bg,
                divider_color,
                accent,
                fg,
                &tabs,
                &theme,
                base_sz,
                small_sz,
                ds.typography.large_size,
                cx,
            ))
            // ---- Footer ----
            .child(Self::render_footer(
                footer_h,
                footer_bg,
                selected_id,
                accent,
                &theme,
                small_sz,
                ds.spacing.card_padding,
                sidebar_pct,
                inspector_pct,
                tabs,
            ))
    }
}

impl ShowcaseView {
    /// Showcase chrome: title bar with window size and axis label.
    ///
    /// Extracted from [`Render::render`] so the root view stays a thin
    /// composition of header / content / footer sections.
    #[allow(clippy::too_many_arguments)]
    fn render_header(
        header_h: f32,
        header_bg: Rgba,
        selected_id: &str,
        accent: Rgba,
        theme: &ShowcaseTheme,
        ds: &gpui_design::DesignSystem,
        base_sz: f32,
        small_sz: f32,
        w: f32,
        h: f32,
        axis_label: &str,
    ) -> impl IntoElement {
        div()
            .h(px(header_h))
            .w_full()
            .bg(header_bg)
            .when(selected_id == "header", |d| {
                d.border_1().border_color(muted(accent, 0.8))
            })
            .flex()
            .flex_row()
            .items_center()
            .px(px(ds.spacing.card_padding))
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(ds.spacing.control_gap + ds.spacing.grid_unit))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(base_sz))
                            .font_weight(FontWeight::BOLD)
                            .text_color(accent)
                            .child("gpui-builder"),
                    )
                    .child(
                        div()
                            .text_size(px(small_sz))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(format!("{w:.0}x{h:.0}  {axis_label}"))),
                    ),
            )
            .child(
                div()
                    .text_size(px(small_sz))
                    .text_color(theme.text_muted)
                    .child("click tree rows | drag dividers | resize window"),
            )
    }

    /// Showcase chrome: status bar with panel ratios and collapse tabs.
    ///
    /// Extracted from [`Render::render`] alongside [`Self::render_header`].
    #[allow(clippy::too_many_arguments)]
    fn render_footer(
        footer_h: f32,
        footer_bg: Rgba,
        selected_id: &str,
        accent: Rgba,
        theme: &ShowcaseTheme,
        small_sz: f32,
        card_padding: f32,
        sidebar_pct: f32,
        inspector_pct: f32,
        tabs: &[(&str, &str)],
    ) -> impl IntoElement {
        div()
            .h(px(footer_h))
            .w_full()
            .bg(footer_bg)
            .when(selected_id == "footer", |d| {
                d.border_1().border_color(muted(accent, 0.8))
            })
            .flex()
            .flex_row()
            .items_center()
            .px(px(card_padding))
            .justify_between()
            .child(
                div()
                    .text_size(px(small_sz))
                    .text_color(theme.text_muted)
                    .child(SharedString::from(format!(
                        "sidebar: {sidebar_pct:.0}%  inspector: {inspector_pct:.0}%"
                    ))),
            )
            .child(if !tabs.is_empty() {
                let labels: Vec<&str> = tabs.iter().map(|(_, l)| *l).collect();
                div()
                    .text_size(px(small_sz))
                    .text_color(accent)
                    .child(SharedString::from(format!("Tabs: {}", labels.join(", "))))
                    .into_any_element()
            } else {
                div().into_any_element()
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_content(
        solved: &SolvedTree,
        selected_id: &str,
        is_h: bool,
        sidebar_ratio: f32,
        inspector_ratio: f32,
        content_w: f32,
        content_h: f32,
        sidebar: gpui_builder::SolvedNodeRef<'_, '_>,
        main_n: gpui_builder::SolvedNodeRef<'_, '_>,
        inspector: gpui_builder::SolvedNodeRef<'_, '_>,
        sidebar_bg: Rgba,
        main_bg: Rgba,
        inspector_bg: Rgba,
        divider_color: Rgba,
        accent: Rgba,
        fg: Rgba,
        tabs: &[(&str, &str)],
        theme: &ShowcaseTheme,
        base_sz: f32,
        small_sz: f32,
        large_sz: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ds = cx.design();
        let visible_divider_count = (sidebar.visible() as u8 + inspector.visible() as u8) as f32;
        let drag_extent =
            (if is_h { content_w } else { content_h } - 6.0 * visible_divider_count).max(1.0);
        let base = div()
            .id("content-area")
            .debug_selector(|| "content-area".to_string())
            .overflow_hidden()
            .min_w_0()
            .min_h_0()
            .when(selected_id == "content", |d| {
                d.border_1().border_color(muted(accent, 0.8))
            });

        if is_h {
            base.h(px(content_h))
                .w_full()
                .flex()
                .flex_row()
                // Sidebar
                .when(sidebar.visible(), |d: Stateful<Div>| {
                    d.child(
                        div()
                            .w(px(sidebar.width()))
                            .h_full()
                            .min_w_0()
                            .overflow_hidden()
                            .when(selected_id == "sidebar", |d| {
                                d.border_1().border_color(muted(accent, 0.8))
                            })
                            .child(panel_box(
                                "Sidebar",
                                &size_label(&sidebar),
                                sidebar_bg,
                                fg,
                                false,
                                accent,
                                base_sz,
                                small_sz,
                                ds.spacing.grid_unit,
                            )),
                    )
                })
                // A divider only exists while both adjoining panels exist.
                .when(sidebar.visible(), |d: Stateful<Div>| {
                    d.child(Self::divider_v(
                        "sidebar",
                        divider_color,
                        accent,
                        drag_extent,
                        sidebar_ratio,
                        cx,
                    ))
                })
                // Main
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .min_w_0()
                        .overflow_hidden()
                        .child(Self::main_panel(
                            main_n,
                            main_bg,
                            fg,
                            tabs,
                            theme,
                            &ds,
                            selected_id == "main",
                            large_sz,
                            small_sz,
                        )),
                )
                // Inspector divider + panel
                .when(inspector.visible(), |d: Stateful<Div>| {
                    d.child(Self::divider_v(
                        "inspector",
                        divider_color,
                        accent,
                        drag_extent,
                        inspector_ratio,
                        cx,
                    ))
                    .child(
                        div()
                            .w(px(inspector.width()))
                            .h_full()
                            .min_w_0()
                            .overflow_hidden()
                            .when(selected_id == "inspector", |d| {
                                d.border_1().border_color(muted(accent, 0.8))
                            })
                            .child(Self::visual_tree_inspector(
                                solved,
                                inspector,
                                selected_id,
                                inspector_bg,
                                fg,
                                theme,
                                &ds,
                                base_sz,
                                small_sz,
                                cx,
                            )),
                    )
                })
                .into_any_element()
        } else {
            base.h(px(content_h))
                .w_full()
                .flex()
                .flex_col()
                .when(sidebar.visible(), |d: Stateful<Div>| {
                    d.child(
                        div()
                            .h(px(sidebar.height()))
                            .w_full()
                            .min_h_0()
                            .overflow_hidden()
                            .when(selected_id == "sidebar", |d| {
                                d.border_1().border_color(muted(accent, 0.8))
                            })
                            .child(panel_box(
                                "Sidebar",
                                &size_label(&sidebar),
                                sidebar_bg,
                                fg,
                                false,
                                accent,
                                base_sz,
                                small_sz,
                                ds.spacing.grid_unit,
                            )),
                    )
                })
                .when(sidebar.visible(), |d: Stateful<Div>| {
                    d.child(Self::divider_h(
                        "sidebar",
                        divider_color,
                        accent,
                        drag_extent,
                        sidebar_ratio,
                        cx,
                    ))
                })
                .child(
                    div()
                        .flex_1()
                        .w_full()
                        .min_h_0()
                        .overflow_hidden()
                        .child(Self::main_panel(
                            main_n,
                            main_bg,
                            fg,
                            tabs,
                            theme,
                            &ds,
                            selected_id == "main",
                            large_sz,
                            small_sz,
                        )),
                )
                .when(inspector.visible(), |d: Stateful<Div>| {
                    d.child(Self::divider_h(
                        "inspector",
                        divider_color,
                        accent,
                        drag_extent,
                        inspector_ratio,
                        cx,
                    ))
                    .child(
                        div()
                            .h(px(inspector.height()))
                            .w_full()
                            .min_h_0()
                            .overflow_hidden()
                            .when(selected_id == "inspector", |d| {
                                d.border_1().border_color(muted(accent, 0.8))
                            })
                            .child(Self::visual_tree_inspector(
                                solved,
                                inspector,
                                selected_id,
                                inspector_bg,
                                fg,
                                theme,
                                &ds,
                                base_sz,
                                small_sz,
                                cx,
                            )),
                    )
                })
                .into_any_element()
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "showcase rendering helper keeps visual styling inputs explicit at call sites"
    )]
    pub(super) fn main_panel(
        node: gpui_builder::SolvedNodeRef<'_, '_>,
        bg: Rgba,
        fg: Rgba,
        tabs: &[(&str, &str)],
        theme: &ShowcaseTheme,
        ds: &gpui_design::DesignSystem,
        selected: bool,
        large_sz: f32,
        small_sz: f32,
    ) -> impl IntoElement {
        let mut el = div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(bg)
            .when(selected, |d| {
                d.border_1().border_color(muted(theme.accent, 0.8))
            })
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(ds.spacing.control_gap))
            .child(
                div()
                    .text_size(px(large_sz))
                    .font_weight(FontWeight::BOLD)
                    .text_color(fg)
                    .child("Main Content"),
            )
            .child(
                div()
                    .text_size(px(small_sz))
                    .text_color(muted(fg, 0.5))
                    .child(SharedString::from(format!(
                        "{:.0} x {:.0}",
                        node.width(),
                        node.height()
                    ))),
            );

        if !tabs.is_empty() {
            el = el.child(
                div()
                    .mt(px(ds.spacing.section_gap))
                    .flex()
                    .flex_row()
                    .gap(px(ds.spacing.control_gap))
                    .children(tabs.iter().map(|(_, label)| {
                        div()
                            .px(px(ds.spacing.control_padding_x))
                            .py(px(ds.spacing.control_padding_y * 0.5))
                            .rounded(px(ds.corners.md))
                            .bg(muted(theme.accent, 0.15))
                            .border_1()
                            .border_color(muted(theme.accent, 0.3))
                            .text_size(px(small_sz))
                            .text_color(theme.accent)
                            .child(SharedString::from(label.to_string()))
                    })),
            );
        }

        el
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn visual_tree_inspector(
        solved: &SolvedTree,
        panel: gpui_builder::SolvedNodeRef<'_, '_>,
        selected_id: &str,
        bg: Rgba,
        fg: Rgba,
        theme: &ShowcaseTheme,
        ds: &gpui_design::DesignSystem,
        base_sz: f32,
        small_sz: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut rows = Vec::new();
        collect_visual_tree_rows(solved, &mut rows);
        let node_count = rows.len();
        let selected_size = solved
            .find(selected_id)
            .map(|node| size_label(&node))
            .unwrap_or_default();
        let tree_rows: Vec<AnyElement> = rows
            .into_iter()
            .map(|row| {
                Self::visual_tree_row(row, selected_id, theme, ds, small_sz, cx).into_any_element()
            })
            .collect();

        div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(bg)
            .flex()
            .flex_col()
            .gap(px(ds.spacing.control_gap))
            .p(px(ds.spacing.control_gap))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(base_sz))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(fg)
                                    .child("Visual Tree"),
                            )
                            .child(
                                div()
                                    .text_size(px(small_sz))
                                    .text_color(theme.text_muted)
                                    .child(SharedString::from(format!(
                                        "{node_count} solved nodes"
                                    ))),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(small_sz))
                            .text_color(muted(theme.accent, 0.9))
                            .child(SharedString::from(size_label(&panel))),
                    ),
            )
            .child(
                div()
                    .rounded(px(ds.corners.sm))
                    .border_1()
                    .border_color(theme.border)
                    .bg(muted(theme.background, 0.55))
                    .px(px(ds.spacing.control_gap))
                    .py(px(ds.spacing.control_padding_y * 0.75))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(small_sz))
                            .text_color(theme.text_muted)
                            .child("Selected"),
                    )
                    .child(div().text_size(px(base_sz)).text_color(theme.accent).child(
                        SharedString::from(format!("{selected_id}  {selected_size}")),
                    )),
            )
            .child(
                div()
                    .id("visual-tree-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .children(tree_rows),
            )
    }

    pub(super) fn visual_tree_row(
        row: VisualTreeRow,
        selected_id: &str,
        theme: &ShowcaseTheme,
        ds: &gpui_design::DesignSystem,
        small_sz: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = row.id == selected_id;
        let row_id = row.id.clone();
        let row_id_for_key = row.id.clone();
        let label = if row.visible {
            row.id.clone()
        } else {
            format!("{} (collapsed)", row.id)
        };
        let meta = format!(
            "{}x{}  axis={}{}",
            row.width.round(),
            row.height.round(),
            axis_text(row.resolved_axis),
            row.active_tier
                .as_deref()
                .map(|tier| format!("  tier={tier}"))
                .unwrap_or_default()
        );
        let indent = row.depth as f32 * 14.0;

        div()
            .id(SharedString::from(format!("tree-row-{}", row.id)))
            .debug_selector(|| format!("tree-row-{}", row.id))
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(is_selected)
            .tab_index(0)
            .rounded(px(ds.corners.sm))
            .px(px(ds.spacing.control_padding_x * 0.75))
            .py(px(ds.spacing.control_padding_y * 0.65))
            .bg(if is_selected {
                muted(theme.accent, 0.18)
            } else {
                rgba(0x00000000)
            })
            .border_1()
            .border_color(if is_selected {
                muted(theme.accent, 0.55)
            } else {
                rgba(0x00000000)
            })
            .hover(|s| {
                s.bg(muted(theme.accent, 0.10))
                    .border_color(muted(theme.accent, 0.25))
                    .cursor_pointer()
            })
            .focus_visible(|s| s.border_2().border_color(theme.accent))
            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                view.selected_node = Some(row_id.clone());
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    view.selected_node = Some(row_id_for_key.clone());
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                div()
                    .ml(px(indent))
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .text_size(px(small_sz))
                            .text_color(if row.visible {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            })
                            .child(SharedString::from(label)),
                    )
                    .child(
                        div()
                            .text_size(px((small_sz - 1.0).max(10.0)))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(meta)),
                    ),
            )
    }

    pub(super) fn divider_v(
        panel: &str,
        bg: Rgba,
        hover_bg: Rgba,
        drag_extent: f32,
        ratio: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = SharedString::from(format!("div-v-{panel}"));
        let is_sidebar = panel == "sidebar";
        let target = if is_sidebar {
            DragTarget::Sidebar
        } else {
            DragTarget::Inspector
        };
        div()
            .id(id)
            .debug_selector(|| format!("div-v-{panel}"))
            .role(Role::Slider)
            .aria_label(if is_sidebar {
                "Sidebar panel width"
            } else {
                "Inspector panel width"
            })
            .aria_numeric_value(f64::from(ratio))
            .aria_min_numeric_value(0.08)
            .aria_max_numeric_value(0.45)
            .aria_numeric_value_step(0.02)
            .tab_index(0)
            .w(px(6.0))
            .h_full()
            .flex_shrink_0()
            .bg(bg)
            .hover(move |s| s.bg(hover_bg))
            .focus_visible(move |s| s.bg(hover_bg).border_2().border_color(hover_bg))
            .cursor_col_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                    let start_pos: f32 = event.position.x.into();
                    view.begin_drag(target, Axis::Horizontal, start_pos, drag_extent);
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                let movement = match event.keystroke.key.as_str() {
                    "left" => -0.02,
                    "right" => 0.02,
                    _ => return,
                };
                if view.nudge_ratio(target, Axis::Horizontal, movement) {
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
    }

    pub(super) fn divider_h(
        panel: &str,
        bg: Rgba,
        hover_bg: Rgba,
        drag_extent: f32,
        ratio: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = SharedString::from(format!("div-h-{panel}"));
        let is_sidebar = panel == "sidebar";
        let target = if is_sidebar {
            DragTarget::Sidebar
        } else {
            DragTarget::Inspector
        };
        div()
            .id(id)
            .debug_selector(|| format!("div-h-{panel}"))
            .role(Role::Slider)
            .aria_label(if is_sidebar {
                "Sidebar panel height"
            } else {
                "Inspector panel height"
            })
            .aria_numeric_value(f64::from(ratio))
            .aria_min_numeric_value(0.08)
            .aria_max_numeric_value(0.45)
            .aria_numeric_value_step(0.02)
            .tab_index(0)
            .h(px(6.0))
            .w_full()
            .flex_shrink_0()
            .bg(bg)
            .hover(move |s| s.bg(hover_bg))
            .focus_visible(move |s| s.bg(hover_bg).border_2().border_color(hover_bg))
            .cursor_row_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                    let start_pos: f32 = event.position.y.into();
                    view.begin_drag(target, Axis::Vertical, start_pos, drag_extent);
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                let movement = match event.keystroke.key.as_str() {
                    "up" => -0.02,
                    "down" => 0.02,
                    _ => return,
                };
                if view.nudge_ratio(target, Axis::Vertical, movement) {
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
    }
}

#[cfg(test)]
#[path = "showcase_view_tests.rs"]
mod tests;
