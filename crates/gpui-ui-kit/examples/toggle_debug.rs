//! Toggle Debug Example
//!
//! Demonstrates the Toggle component:
//! - Sliding and Segmented styles
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ToggleDebug {
    sliding_off: bool,
    sliding_on: bool,
    segmented_off: bool,
    segmented_on: bool,
    sm_checked: bool,
    md_checked: bool,
    lg_checked: bool,
    entity: Entity<Self>,
}

impl Render for ToggleDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("toggle-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Toggle Debug"))
            // Sliding style
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sliding Style (Default)").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-off")
                            .checked(self.sliding_off)
                            .label("Off")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.sliding_off = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Toggle::new("toggle-on")
                            .checked(self.sliding_on)
                            .label("On")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.sliding_on = checked;
                                    });
                                }
                            }),
                    ),
            )
            // Segmented style
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Segmented Style").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-seg-off")
                            .checked(self.segmented_off)
                            .style(ToggleStyle::Segmented)
                            .label("Bypass")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.segmented_off = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Toggle::new("toggle-seg-on")
                            .checked(self.segmented_on)
                            .style(ToggleStyle::Segmented)
                            .label("Active")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.segmented_on = checked;
                                    });
                                }
                            }),
                    ),
            )
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-sm")
                            .checked(self.sm_checked)
                            .size(ToggleSize::Sm)
                            .label("Small")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.sm_checked = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Toggle::new("toggle-md")
                            .checked(self.md_checked)
                            .size(ToggleSize::Md)
                            .label("Medium")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.md_checked = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Toggle::new("toggle-lg")
                            .checked(self.lg_checked)
                            .size(ToggleSize::Lg)
                            .label("Large")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.lg_checked = checked;
                                    });
                                }
                            }),
                    ),
            )
            // Disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-dis-off")
                            .disabled(true)
                            .label("Disabled off"),
                    )
                    .child(
                        Toggle::new("toggle-dis-on")
                            .checked(true)
                            .disabled(true)
                            .label("Disabled on"),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Toggle Debug")
            .size(500.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|cx| ToggleDebug {
                sliding_off: false,
                sliding_on: true,
                segmented_off: false,
                segmented_on: true,
                sm_checked: true,
                md_checked: true,
                lg_checked: true,
                entity: cx.entity().clone(),
            })
        },
    );
}
