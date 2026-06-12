//! StepIndicator Debug Example
//!
//! Demonstrates the StepIndicator component:
//! - Horizontal and vertical orientations
//! - Different step statuses

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct StepIndicatorDebug;

impl Render for StepIndicatorDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("step-indicator-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("StepIndicator Debug"))
            // Horizontal
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Horizontal").weight(TextWeight::Bold))
                    .child(
                        StepIndicator::new(
                            "steps-horiz",
                            vec![
                                StepItem::new("Select Speaker").status(StepItemStatus::Completed),
                                StepItem::new("Configure EQ").status(StepItemStatus::Active),
                                StepItem::new("Optimize").status(StepItemStatus::NotVisited),
                                StepItem::new("Export").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .orientation(StepOrientation::Horizontal),
                    ),
            )
            // Vertical
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Vertical").weight(TextWeight::Bold))
                    .child(
                        StepIndicator::new(
                            "steps-vert",
                            vec![
                                StepItem::new("Download Measurements")
                                    .status(StepItemStatus::Completed),
                                StepItem::new("Run Optimization").status(StepItemStatus::Completed),
                                StepItem::new("Review Results").status(StepItemStatus::Active),
                                StepItem::new("Apply Filters").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .orientation(StepOrientation::Vertical),
                    ),
            )
            // With error
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Error State").weight(TextWeight::Bold))
                    .child(
                        StepIndicator::new(
                            "steps-error",
                            vec![
                                StepItem::new("Connect Device").status(StepItemStatus::Completed),
                                StepItem::new("Calibrate").status(StepItemStatus::Error),
                                StepItem::new("Measure").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .orientation(StepOrientation::Horizontal),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("StepIndicator Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| StepIndicatorDebug),
    );
}
