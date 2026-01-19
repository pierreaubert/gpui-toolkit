//! Input Debug Example
//!
//! A minimal example to test the self-contained text input component:
//! 1. Click to focus and start editing (handled internally by Input)
//! 2. Type to enter text (handled internally by Input)
//! 3. Enter to confirm, Escape to cancel (handled internally by Input)
//! 4. Parent only needs to handle value changes via callbacks
//!
//! The Input component now handles all focus and keyboard events internally.
//! Parent components only need to provide callbacks for value changes.

use gpui::*;
use gpui_ui_kit::input::{Input, InputSize, InputVariant};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct InputDebug {
    // Just store the confirmed values - Input handles editing internally
    input1_value: String,
    input2_value: String,
    input3_value: String,
    input4_value: String,

    // For live text display (optional - shows current edit text)
    input1_live_text: String,
    input2_live_text: String,

    entity: Entity<Self>,
}

impl InputDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            input1_value: "Hello World".to_string(),
            input2_value: String::new(),
            input3_value: "Filled variant".to_string(),
            input4_value: "Flushed variant".to_string(),

            input1_live_text: String::new(),
            input2_live_text: String::new(),

            entity: cx.entity().clone(),
        }
    }
}

impl Render for InputDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("input-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Heading::h1("Text Input Debug (Self-Contained)"))
                    .child(Text::new(
                        "The Input component now handles all focus and keyboard events internally!",
                    )),
            )
            .child(Divider::new().build())
            // Instructions
            .child(
                div()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .p_4()
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("How to use:").weight(TextWeight::Bold))
                            .child(Text::new("1. Click on an input field to start editing"))
                            .child(Text::new(
                                "2. Type to enter text (full cursor navigation supported)",
                            ))
                            .child(Text::new("3. Press Enter to confirm your changes"))
                            .child(Text::new(
                                "4. Press Escape to cancel and restore original value",
                            ))
                            .child(
                                Text::new("5. Emacs keybindings: Ctrl+A/E/K/U/W/H/D/F/B")
                                    .muted(true),
                            ),
                    ),
            )
            // Basic input with value
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Text::new("Default Input (with initial value):").weight(TextWeight::Medium),
                    )
                    .child({
                        let value = self.input1_value.clone();
                        let entity = entity.clone();

                        div().w(px(300.0)).child(
                            Input::new("input-1")
                                .value(value)
                                .placeholder("Enter some text...")
                                .label("Username")
                                .size(InputSize::Md)
                                .on_change({
                                    let entity = entity.clone();
                                    move |new_value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input1_value = new_value.to_string();
                                        });
                                    }
                                })
                                .on_text_change({
                                    let entity = entity.clone();
                                    move |text, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input1_live_text = text;
                                        });
                                    }
                                }),
                        )
                    })
                    .child(
                        Text::new(format!(
                            "Confirmed: \"{}\" | Live: \"{}\"",
                            self.input1_value, self.input1_live_text
                        ))
                        .size(TextSize::Sm)
                        .muted(true),
                    ),
            )
            // Empty input with placeholder
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Empty Input (with placeholder):").weight(TextWeight::Medium))
                    .child({
                        let value = self.input2_value.clone();
                        let entity = entity.clone();

                        div().w(px(300.0)).child(
                            Input::new("input-2")
                                .value(value)
                                .placeholder("Type something here...")
                                .label("Email")
                                .size(InputSize::Md)
                                .on_change({
                                    let entity = entity.clone();
                                    move |new_value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input2_value = new_value.to_string();
                                        });
                                    }
                                })
                                .on_text_change({
                                    let entity = entity.clone();
                                    move |text, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input2_live_text = text;
                                        });
                                    }
                                }),
                        )
                    })
                    .child(
                        Text::new(format!(
                            "Confirmed: {} | Live: \"{}\"",
                            if self.input2_value.is_empty() {
                                "(empty)".to_string()
                            } else {
                                format!("\"{}\"", self.input2_value)
                            },
                            self.input2_live_text
                        ))
                        .size(TextSize::Sm)
                        .muted(true),
                    ),
            )
            .child(Divider::new().build())
            // Variants section
            .child(Heading::h2("Input Variants"))
            // Filled variant
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Filled Variant:").weight(TextWeight::Medium))
                    .child({
                        let value = self.input3_value.clone();
                        let entity = entity.clone();

                        div().w(px(300.0)).child(
                            Input::new("input-3")
                                .value(value)
                                .placeholder("Filled input...")
                                .variant(InputVariant::Filled)
                                .on_change({
                                    move |new_value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input3_value = new_value.to_string();
                                        });
                                    }
                                }),
                        )
                    }),
            )
            // Flushed variant
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Text::new("Flushed Variant (bottom border only):")
                            .weight(TextWeight::Medium),
                    )
                    .child({
                        let value = self.input4_value.clone();
                        let entity = entity.clone();

                        div().w(px(300.0)).child(
                            Input::new("input-4")
                                .value(value)
                                .placeholder("Flushed input...")
                                .variant(InputVariant::Flushed)
                                .on_change({
                                    move |new_value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.input4_value = new_value.to_string();
                                        });
                                    }
                                }),
                        )
                    }),
            )
            .child(Divider::new().build())
            // Sizes section
            .child(Heading::h2("Input Sizes"))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .items_end()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Small").size(TextSize::Sm))
                            .child(
                                div().w(px(150.0)).child(
                                    Input::new("size-sm")
                                        .value("Small size")
                                        .size(InputSize::Sm)
                                        .readonly(true),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Medium").size(TextSize::Sm))
                            .child(
                                div().w(px(150.0)).child(
                                    Input::new("size-md")
                                        .value("Medium size")
                                        .size(InputSize::Md)
                                        .readonly(true),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Large").size(TextSize::Sm))
                            .child(
                                div().w(px(150.0)).child(
                                    Input::new("size-lg")
                                        .value("Large size")
                                        .size(InputSize::Lg)
                                        .readonly(true),
                                ),
                            ),
                    ),
            )
            .child(Divider::new().build())
            // States section
            .child(Heading::h2("Input States"))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Disabled").size(TextSize::Sm))
                            .child(
                                div().w(px(200.0)).child(
                                    Input::new("state-disabled")
                                        .value("Cannot edit")
                                        .disabled(true),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Readonly").size(TextSize::Sm))
                            .child(
                                div().w(px(200.0)).child(
                                    Input::new("state-readonly")
                                        .value("Read only text")
                                        .readonly(true),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("With Error").size(TextSize::Sm))
                            .child(
                                div().w(px(200.0)).child(
                                    Input::new("state-error")
                                        .value("Invalid value")
                                        .error("This field has an error")
                                        .readonly(true),
                                ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Input Debug")
            .size(800.0, 900.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(InputDebug::new),
    );
}
