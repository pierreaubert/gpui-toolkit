//! Tooltip Debug Example
//!
//! Demonstrates the Tooltip component:
//! - Different placements
//! - WithTooltip wrapper

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct TooltipDebug;

impl Render for TooltipDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("tooltip-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Tooltip Debug"))
            .child(Text::new("Hover over elements to see tooltips").muted(true))
            // Placements
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Placements").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                WithTooltip::new(Button::new("tt-top", "Top"), "Tooltip on top")
                                    .placement(TooltipPlacement::Top),
                            )
                            .child(
                                WithTooltip::new(
                                    Button::new("tt-bottom", "Bottom"),
                                    "Tooltip on bottom",
                                )
                                .placement(TooltipPlacement::Bottom),
                            )
                            .child(
                                WithTooltip::new(Button::new("tt-left", "Left"), "Tooltip on left")
                                    .placement(TooltipPlacement::Left),
                            )
                            .child(
                                WithTooltip::new(
                                    Button::new("tt-right", "Right"),
                                    "Tooltip on right",
                                )
                                .placement(TooltipPlacement::Right),
                            ),
                    ),
            )
            // On different elements
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("On Various Elements").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(WithTooltip::new(Badge::new("3"), "3 notifications"))
                            .child(WithTooltip::new(
                                Text::new("Hover me"),
                                "This is a text element",
                            )),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Tooltip Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| TooltipDebug),
    );
}
