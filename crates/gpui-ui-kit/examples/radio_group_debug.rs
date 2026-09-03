//! RadioGroup Debug Example
//!
//! Demonstrates the RadioGroup component:
//! - Single selection with on_change
//! - Vertical and horizontal orientations
//! - All sizes, disabled group and disabled option

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::radio_group::{
    RadioGroup, RadioGroupOrientation, RadioGroupSize, RadioOption,
};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct RadioGroupDebug {
    selected: Option<SharedString>,
    horizontal: Option<SharedString>,
    entity: Entity<Self>,
}

impl RadioGroupDebug {
    fn options() -> Vec<RadioOption> {
        vec![
            RadioOption::new("a", "Alpha"),
            RadioOption::new("b", "Beta"),
            RadioOption::new("c", "Gamma (disabled)").disabled(true),
        ]
    }
}

impl Render for RadioGroupDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("radio-group-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("RadioGroup Debug"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Vertical").weight(TextWeight::Bold))
                    .child(
                        RadioGroup::new("rg-vertical")
                            .options(Self::options())
                            .selected(self.selected.clone())
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.selected = Some(value);
                                    });
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Horizontal").weight(TextWeight::Bold))
                    .child(
                        RadioGroup::new("rg-horizontal")
                            .options(Self::options())
                            .orientation(RadioGroupOrientation::Horizontal)
                            .selected(self.horizontal.clone())
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.horizontal = Some(value);
                                    });
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(
                        RadioGroup::new("rg-sm")
                            .options(Self::options())
                            .size(RadioGroupSize::Sm)
                            .selected(self.selected.clone()),
                    )
                    .child(
                        RadioGroup::new("rg-lg")
                            .options(Self::options())
                            .size(RadioGroupSize::Lg)
                            .selected(self.selected.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        RadioGroup::new("rg-disabled")
                            .options(Self::options())
                            .disabled(true)
                            .selected(Some("a".into())),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("RadioGroup Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|cx| RadioGroupDebug {
                selected: Some("a".into()),
                horizontal: None,
                entity: cx.entity().clone(),
            })
        },
    );
}
