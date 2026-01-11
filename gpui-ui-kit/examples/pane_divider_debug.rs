//! Pane Divider Debug Example
//!
//! Interactive showcase for the PaneDivider component:
//! - Vertical dividers (Left/Right collapse)
//! - Horizontal dividers (Up/Down collapse)
//! - Collapsed/expanded states
//! - Labels when collapsed
//! - Double-click to toggle

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::pane_divider::{CollapseDirection, PaneDivider, PaneDividerTheme};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Demo state
pub struct PaneDividerDebug {
    /// Left panel collapsed
    left_collapsed: bool,
    /// Right panel collapsed
    right_collapsed: bool,
    /// Top panel collapsed
    top_collapsed: bool,
    /// Bottom panel collapsed
    bottom_collapsed: bool,
    /// Entity reference
    entity: Entity<Self>,
}

impl PaneDividerDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            left_collapsed: false,
            right_collapsed: false,
            top_collapsed: false,
            bottom_collapsed: false,
            entity: cx.entity().clone(),
        }
    }

    /// Render a sample panel
    fn render_panel(title: impl Into<SharedString>, bg: Rgba, theme: &Theme) -> impl IntoElement {
        div()
            .flex_1()
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
}

impl Render for PaneDividerDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        // Custom theme for dividers
        let divider_theme = PaneDividerTheme {
            background: theme.surface,
            background_hover: theme.surface_hover,
            background_collapsed: theme.muted,
            foreground: theme.text_muted,
            foreground_hover: theme.text_primary,
            border: theme.border,
        };

        div()
            .id("pane-divider-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("Pane Divider Component Debug"))
                    .child(
                        Text::new("Double-click dividers to collapse/expand. Click collapsed divider to expand. Arrows show collapse direction.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar - demonstrates language switching works
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
                    .child(Text::new(format!("Left: {}", if self.left_collapsed { "Collapsed" } else { "Expanded" })).size(TextSize::Sm))
                    .child(Text::new(format!("Right: {}", if self.right_collapsed { "Collapsed" } else { "Expanded" })).size(TextSize::Sm))
                    .child(Text::new(format!("Top: {}", if self.top_collapsed { "Collapsed" } else { "Expanded" })).size(TextSize::Sm))
                    .child(Text::new(format!("Bottom: {}", if self.bottom_collapsed { "Collapsed" } else { "Expanded" })).size(TextSize::Sm)),
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
                            // Left panel (or collapsed divider)
                            .when(!self.left_collapsed, |d| {
                                d.child(Self::render_panel("Left Panel", theme.muted, &theme))
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
                                    }),
                            )
                            // Right panel (or collapsed divider)
                            .when(!self.right_collapsed, |d| {
                                d.child(Self::render_panel("Right Panel", theme.muted, &theme))
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
                            // Top panel (or collapsed divider)
                            .when(!self.top_collapsed, |d| {
                                d.child(Self::render_panel("Top Panel", theme.muted, &theme))
                            })
                            // Top divider (collapses up)
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
                                    }),
                            )
                            // Middle panel
                            .child(Self::render_panel("Middle Panel", theme.background, &theme))
                            // Bottom divider (collapses down)
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
                                    }),
                            )
                            // Bottom panel (or collapsed divider)
                            .when(!self.bottom_collapsed, |d| {
                                d.child(Self::render_panel("Bottom Panel", theme.muted, &theme))
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
                            .child(Text::new("- Double-click a divider to collapse the adjacent panel").size(TextSize::Sm))
                            .child(Text::new("- Click a collapsed divider to expand it").size(TextSize::Sm))
                            .child(Text::new("- Arrows indicate the collapse direction").size(TextSize::Sm))
                            .child(Text::new("- Collapsed dividers show vertical text labels").size(TextSize::Sm)),
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
