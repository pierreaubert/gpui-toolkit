//! Checkbox Debug Example
//!
//! Demonstrates the Checkbox component:
//! - Checked, unchecked, indeterminate states
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct CheckboxDebug {
    unchecked: bool,
    checked: bool,
    indeterminate_checked: bool,
    indeterminate: bool,
    sm_checked: bool,
    md_checked: bool,
    lg_checked: bool,
    entity: Entity<Self>,
}

impl Render for CheckboxDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("checkbox-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Checkbox Debug"))
            // States
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("States").weight(TextWeight::Bold))
                    .child(
                        Checkbox::new("cb-unchecked")
                            .checked(self.unchecked)
                            .label("Unchecked")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.unchecked = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Checkbox::new("cb-checked")
                            .checked(self.checked)
                            .label("Checked")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.checked = checked;
                                    });
                                }
                            }),
                    )
                    .child(
                        Checkbox::new("cb-indeterminate")
                            .checked(self.indeterminate_checked)
                            .indeterminate(self.indeterminate)
                            .label("Indeterminate")
                            .on_change({
                                let entity = entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.indeterminate = false;
                                        this.indeterminate_checked = checked;
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
                        Checkbox::new("cb-sm")
                            .checked(self.sm_checked)
                            .size(CheckboxSize::Sm)
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
                        Checkbox::new("cb-md")
                            .checked(self.md_checked)
                            .size(CheckboxSize::Md)
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
                        Checkbox::new("cb-lg")
                            .checked(self.lg_checked)
                            .size(CheckboxSize::Lg)
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
                        Checkbox::new("cb-dis-off")
                            .disabled(true)
                            .label("Disabled unchecked"),
                    )
                    .child(
                        Checkbox::new("cb-dis-on")
                            .checked(true)
                            .disabled(true)
                            .label("Disabled checked"),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Checkbox Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|cx| CheckboxDebug {
                unchecked: false,
                checked: true,
                indeterminate_checked: false,
                indeterminate: true,
                sm_checked: true,
                md_checked: true,
                lg_checked: true,
                entity: cx.entity().clone(),
            })
        },
    );
}
