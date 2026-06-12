//! Pane Divider Debug Example
//!
//! Interactive showcase for the PaneDivider component:
//! - Vertical dividers (Left/Right collapse)
//! - Horizontal dividers (Up/Down collapse)
//! - Collapsed/expanded states
//! - Labels when collapsed
//! - Double-click to toggle
//! - Drag to resize panels

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::pane_divider::{CollapseDirection, PaneDivider, PaneDividerTheme};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Which divider is currently being dragged
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragTarget {
    Left,
    Right,
    Top,
    Bottom,
}

/// Demo state
pub struct PaneDividerDebug {
    left_collapsed: bool,
    right_collapsed: bool,
    top_collapsed: bool,
    bottom_collapsed: bool,
    left_width: f32,
    right_width: f32,
    top_height: f32,
    bottom_height: f32,
    /// Active drag state: which divider, start mouse pos, panel size at drag start
    drag: Option<(DragTarget, f32, f32)>,
    entity: Entity<Self>,
}

const DEFAULT_PANEL_SIZE: f32 = 150.0;
const MIN_PANEL_SIZE: f32 = 50.0;
const MAX_PANEL_SIZE: f32 = 400.0;

impl PaneDividerDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            left_collapsed: false,
            right_collapsed: false,
            top_collapsed: false,
            bottom_collapsed: false,
            left_width: DEFAULT_PANEL_SIZE,
            right_width: DEFAULT_PANEL_SIZE,
            top_height: DEFAULT_PANEL_SIZE / 2.0,
            bottom_height: DEFAULT_PANEL_SIZE / 2.0,
            drag: None,
            entity: cx.entity().clone(),
        }
    }

    fn render_panel(title: impl Into<SharedString>, bg: Rgba, theme: &Theme) -> Div {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .min_w_0()
            .min_h_0()
            .bg(bg)
            .child(
                Text::new(title)
                    .weight(TextWeight::Bold)
                    .color(theme.text_primary),
            )
    }

    fn render_sized_panel(title: impl Into<SharedString>, bg: Rgba, theme: &Theme) -> Div {
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(bg)
            .child(
                Text::new(title)
                    .weight(TextWeight::Bold)
                    .color(theme.text_primary),
            )
    }

    fn handle_drag_move(&mut self, position: Point<Pixels>) {
        let Some((target, start_pos, start_size)) = self.drag else {
            return;
        };
        match target {
            DragTarget::Left => {
                let delta: f32 = position.x.into();
                self.left_width =
                    (start_size + (delta - start_pos)).clamp(MIN_PANEL_SIZE, MAX_PANEL_SIZE);
            }
            DragTarget::Right => {
                let delta: f32 = position.x.into();
                // Dragging right = shrinking the right panel
                self.right_width =
                    (start_size - (delta - start_pos)).clamp(MIN_PANEL_SIZE, MAX_PANEL_SIZE);
            }
            DragTarget::Top => {
                let delta: f32 = position.y.into();
                self.top_height =
                    (start_size + (delta - start_pos)).clamp(MIN_PANEL_SIZE, MAX_PANEL_SIZE);
            }
            DragTarget::Bottom => {
                let delta: f32 = position.y.into();
                self.bottom_height =
                    (start_size - (delta - start_pos)).clamp(MIN_PANEL_SIZE, MAX_PANEL_SIZE);
            }
        }
    }
}

impl Render for PaneDividerDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        let divider_theme = PaneDividerTheme {
            background: theme.surface,
            background_hover: theme.surface_hover,
            background_collapsed: theme.muted,
            foreground: theme.text_muted,
            foreground_hover: theme.text_primary,
            border: theme.border,
            tint: Rgba {
                a: 0.42,
                ..theme.accent
            },
            tint_hover: theme.accent,
        };

        let mut root = div()
            .id("pane-divider-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6();

        // Mouse move/up on root for drag tracking (use cx.listener like app-gpui)
        root = root.on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
            if this.drag.is_some() {
                this.handle_drag_move(event.position);
                cx.notify();
            }
        }));
        root = root.on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _window, _cx| {
                this.drag = None;
            }),
        );

        root
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("Pane Divider Component Debug"))
                    .child(
                        Text::new("Drag dividers to resize panels. Double-click to collapse/expand. Click collapsed divider to expand.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("🌐 {}: ", cx.t(TranslationKey::MenuLanguage))).weight(TextWeight::Medium))
                    .child(Text::new(cx.language().native_name()).color(theme.accent))
                    .child(Text::new(" | "))
                    .child(Text::new(cx.t(TranslationKey::SectionLayout)).color(theme.text_secondary)),
            )
            // Status bar
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("Left: {} ({:.0}px)", if self.left_collapsed { "Collapsed" } else { "Expanded" }, self.left_width)).size(TextSize::Sm))
                    .child(Text::new(format!("Right: {} ({:.0}px)", if self.right_collapsed { "Collapsed" } else { "Expanded" }, self.right_width)).size(TextSize::Sm))
                    .child(Text::new(format!("Top: {} ({:.0}px)", if self.top_collapsed { "Collapsed" } else { "Expanded" }, self.top_height)).size(TextSize::Sm))
                    .child(Text::new(format!("Bottom: {} ({:.0}px)", if self.bottom_collapsed { "Collapsed" } else { "Expanded" }, self.bottom_height)).size(TextSize::Sm))
                    .child(Text::new(format!("Dragging: {}", if self.drag.is_some() { "Yes" } else { "No" })).size(TextSize::Sm).color(if self.drag.is_some() { theme.accent } else { theme.text_muted })),
            )
            // Vertical Dividers Demo
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Vertical Dividers (Left/Right)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Sit between left and right panels").size(TextSize::Sm).muted(true))
                    .child(
                        div()
                            .h(px(200.0))
                            .flex()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .overflow_hidden()
                            // Left panel
                            .when(!self.left_collapsed, |d| {
                                d.child(Self::render_sized_panel("Left Panel", theme.muted, &theme).w(px(self.left_width)).h_full())
                            })
                            // Left divider
                            .child(
                                PaneDivider::vertical("left-divider", CollapseDirection::Left)
                                    .label("Left")
                                    .collapsed(self.left_collapsed)
                                    .theme(divider_theme.clone())
                                    .on_toggle({
                                        let entity = entity.clone();
                                        move |collapsed, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.left_collapsed = collapsed;
                                            });
                                        }
                                    })
                                    .on_drag_start({
                                        let entity = entity.clone();
                                        move |pos, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.drag = Some((DragTarget::Left, pos, this.left_width));
                                            });
                                        }
                                    }),
                            )
                            // Center panel
                            .child(Self::render_panel("Center Panel", theme.background, &theme))
                            // Right divider
                            .child(
                                PaneDivider::vertical("right-divider", CollapseDirection::Right)
                                    .label("Right")
                                    .collapsed(self.right_collapsed)
                                    .theme(divider_theme.clone())
                                    .on_toggle({
                                        let entity = entity.clone();
                                        move |collapsed, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.right_collapsed = collapsed;
                                            });
                                        }
                                    })
                                    .on_drag_start({
                                        let entity = entity.clone();
                                        move |pos, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.drag = Some((DragTarget::Right, pos, this.right_width));
                                            });
                                        }
                                    }),
                            )
                            // Right panel
                            .when(!self.right_collapsed, |d| {
                                d.child(Self::render_sized_panel("Right Panel", theme.muted, &theme).w(px(self.right_width)).h_full())
                            }),
                    ),
            )
            // Horizontal Dividers Demo
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Horizontal Dividers (Up/Down)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Sit between top and bottom panels").size(TextSize::Sm).muted(true))
                    .child(
                        div()
                            .h(px(300.0))
                            .flex()
                            .flex_col()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .overflow_hidden()
                            // Top panel
                            .when(!self.top_collapsed, |d| {
                                d.child(Self::render_sized_panel("Top Panel", theme.muted, &theme).h(px(self.top_height)).w_full())
                            })
                            // Top divider
                            .child(
                                PaneDivider::horizontal("top-divider", CollapseDirection::Up)
                                    .label("Top")
                                    .collapsed(self.top_collapsed)
                                    .theme(divider_theme.clone())
                                    .on_toggle({
                                        let entity = entity.clone();
                                        move |collapsed, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.top_collapsed = collapsed;
                                            });
                                        }
                                    })
                                    .on_drag_start({
                                        let entity = entity.clone();
                                        move |pos, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.drag = Some((DragTarget::Top, pos, this.top_height));
                                            });
                                        }
                                    }),
                            )
                            // Middle panel
                            .child(Self::render_panel("Middle Panel", theme.background, &theme))
                            // Bottom divider
                            .child(
                                PaneDivider::horizontal("bottom-divider", CollapseDirection::Down)
                                    .label("Bottom")
                                    .collapsed(self.bottom_collapsed)
                                    .theme(divider_theme.clone())
                                    .on_toggle({
                                        let entity = entity.clone();
                                        move |collapsed, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.bottom_collapsed = collapsed;
                                            });
                                        }
                                    })
                                    .on_drag_start({
                                        let entity = entity.clone();
                                        move |pos, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.drag = Some((DragTarget::Bottom, pos, this.bottom_height));
                                            });
                                        }
                                    }),
                            )
                            // Bottom panel
                            .when(!self.bottom_collapsed, |d| {
                                d.child(Self::render_sized_panel("Bottom Panel", theme.muted, &theme).h(px(self.bottom_height)).w_full())
                            }),
                    ),
            )
            // Reset buttons
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Button::new("expand-all", "Expand All")
                            .variant(ButtonVariant::Secondary)
                            .size(ButtonSize::Sm)
                            .on_click({
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, _| {
                                        this.left_collapsed = false;
                                        this.right_collapsed = false;
                                        this.top_collapsed = false;
                                        this.bottom_collapsed = false;
                                        this.left_width = DEFAULT_PANEL_SIZE;
                                        this.right_width = DEFAULT_PANEL_SIZE;
                                        this.top_height = DEFAULT_PANEL_SIZE / 2.0;
                                        this.bottom_height = DEFAULT_PANEL_SIZE / 2.0;
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("collapse-all", "Collapse All")
                            .variant(ButtonVariant::Secondary)
                            .size(ButtonSize::Sm)
                            .on_click({
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, _| {
                                        this.left_collapsed = true;
                                        this.right_collapsed = true;
                                        this.top_collapsed = true;
                                        this.bottom_collapsed = true;
                                    });
                                }
                            }),
                    ),
            )
            // Instructions
            .child(
                div()
                    .p_4()
                    .bg(theme.surface_hover)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Instructions:").weight(TextWeight::Bold))
                            .child(Text::new("- Drag a divider to resize adjacent panels").size(TextSize::Sm))
                            .child(Text::new("- Double-click a divider to collapse the adjacent panel").size(TextSize::Sm))
                            .child(Text::new("- Click a collapsed divider to expand it").size(TextSize::Sm))
                            .child(Text::new("- Panels clamp between 50px and 400px").size(TextSize::Sm)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Pane Divider Debug")
            .size(900.0, 900.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(PaneDividerDebug::new),
    );
}
