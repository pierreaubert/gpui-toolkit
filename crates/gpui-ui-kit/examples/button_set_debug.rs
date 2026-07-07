//! ButtonSet Debug Example
//!
//! Demonstrates the ButtonSet component:
//! - Basic button set with selection
//! - Different sizes
//! - Disabled state

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ButtonSetDebug {
    selected: SharedString,
    xs_selected: SharedString,
    sm_selected: SharedString,
    lg_selected: SharedString,
    toggle_selected: SharedString,
    entity: Entity<Self>,
}

impl Render for ButtonSetDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("button-set-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("ButtonSet Debug"))
            .child(Text::new(format!("Selected: {}", self.selected)).color(theme.accent))
            // Default
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default").weight(TextWeight::Bold))
                    .child(
                        ButtonSet::new("bset-default")
                            .options(vec![
                                ButtonSetOption::new("stereo", "Stereo"),
                                ButtonSetOption::new("surround", "5.0 Surround"),
                                ButtonSetOption::new("atmos", "Atmos"),
                            ])
                            .selected(self.selected.clone())
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.selected = value.clone();
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
                        ButtonSet::new("bset-xs")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(self.xs_selected.clone())
                            .size(ButtonSetSize::Xs)
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.xs_selected = value.clone();
                                    });
                                }
                            }),
                    )
                    .child(
                        ButtonSet::new("bset-sm")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(self.sm_selected.clone())
                            .size(ButtonSetSize::Sm)
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.sm_selected = value.clone();
                                    });
                                }
                            }),
                    )
                    .child(
                        ButtonSet::new("bset-lg")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(self.lg_selected.clone())
                            .size(ButtonSetSize::Lg)
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.lg_selected = value.clone();
                                    });
                                }
                            }),
                    )
                    .child(
                        ButtonSet::new("bset-toggle")
                            .options(vec![
                                ButtonSetOption::new("on", "On"),
                                ButtonSetOption::new("off", "Off"),
                            ])
                            .selected(self.toggle_selected.clone())
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.toggle_selected = value.clone();
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
                    .gap_2()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        ButtonSet::new("bset-disabled")
                            .options(vec![
                                ButtonSetOption::new("on", "On"),
                                ButtonSetOption::new("off", "Off"),
                            ])
                            .selected(SharedString::from("on"))
                            .disabled(true),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("ButtonSet Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|_cx| ButtonSetDebug {
                selected: "stereo".into(),
                xs_selected: "a".into(),
                sm_selected: "b".into(),
                lg_selected: "c".into(),
                toggle_selected: "on".into(),
                entity: _cx.entity().clone(),
            })
        },
    );
}
