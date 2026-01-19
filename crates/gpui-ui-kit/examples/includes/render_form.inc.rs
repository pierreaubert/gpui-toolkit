impl Showcase {
    #[allow(clippy::too_many_arguments)]
    fn render_form_controls_section(
        &self,
        toggle_on: bool,
        toggle_lg: bool,
        checkbox_checked: bool,
        slider_value: f32,
        vertical_slider_value: f64,
        number_value: f64,
        number_freq: f64,
        number_db: f64,
        _editing_number: Option<&'static str>,
        _edit_text: String,
        _text_selected: bool,
        input_value: String,
        _input_editing: bool,
        _input_edit_text: String,
        _input_selected: bool,
        buttonset_view_mode: SharedString,
        buttonset_alignment: SharedString,
        entity: Entity<Self>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionFormControls);
        let toggles_label = cx.t(TranslationKey::LabelToggles);
        let checkboxes_label = cx.t(TranslationKey::LabelCheckboxes);
        let slider_label = cx.t(TranslationKey::LabelSlider);
        let small = cx.t(TranslationKey::LabelSmall);
        let medium = cx.t(TranslationKey::LabelMedium);
        let large = cx.t(TranslationKey::LabelLarge);
        let disabled = cx.t(TranslationKey::LabelDisabled);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Toggles
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(toggles_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(
                                Toggle::new("toggle-sm")
                                    .size(ToggleSize::Sm)
                                    .checked(toggle_on)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.toggle_on = checked;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Toggle::new("toggle-md")
                                    .size(ToggleSize::Md)
                                    .checked(toggle_on)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.toggle_on = checked;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Toggle::new("toggle-lg")
                                    .size(ToggleSize::Lg)
                                    .checked(toggle_lg)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.toggle_lg = checked;
                                            });
                                        }
                                    }),
                            )
                            .child(Toggle::new("toggle-disabled").disabled(true).checked(true)),
                    ),
            )
            // Checkboxes
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(checkboxes_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(
                                Checkbox::new("cb-sm")
                                    .label(small)
                                    .size(CheckboxSize::Sm)
                                    .checked(checkbox_checked)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.checkbox_checked = checked;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Checkbox::new("cb-md")
                                    .label(medium)
                                    .size(CheckboxSize::Md)
                                    .checked(checkbox_checked)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.checkbox_checked = checked;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Checkbox::new("cb-lg")
                                    .label(large)
                                    .size(CheckboxSize::Lg)
                                    .checked(!checkbox_checked)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.checkbox_checked = !checked;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Checkbox::new("cb-disabled")
                                    .label(disabled)
                                    .disabled(true)
                                    .checked(true),
                            ),
                    ),
            )
            // Slider
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Text::new(format!("{}: {:.0}%", slider_label, slider_value * 100.0))
                            .weight(TextWeight::Medium),
                    )
                    .child(
                        div().w(px(300.0)).child(
                            Slider::new("slider-demo")
                                .value(slider_value)
                                .min(0.0)
                                .max(1.0)
                                .size(SliderSize::Md)
                                .on_change({
                                    let entity = entity.clone();
                                    move |value, _window, cx| {
                                        entity.update(cx, |showcase, _cx| {
                                            showcase.slider_value = value;
                                        });
                                    }
                                }),
                        ),
                    ),
            )
            // Number Input
            // Note: NumberInput now handles editing state internally - just provide value and on_change
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Text::new("Number Input (click value to edit)").weight(TextWeight::Medium),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .align(StackAlign::End)
                            .child(
                                NumberInput::new("num-basic")
                                    .value(number_value)
                                    .min(0.0)
                                    .max(100.0)
                                    .step(1.0)
                                    .decimals(0)
                                    .label("Count")
                                    .size(NumberInputSize::Md)
                                    .width(120.0)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.number_value = value;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                NumberInput::new("num-freq")
                                    .value(number_freq)
                                    .min(20.0)
                                    .max(20000.0)
                                    .step(100.0)
                                    .decimals(0)
                                    .unit("Hz")
                                    .label("Frequency")
                                    .size(NumberInputSize::Md)
                                    .width(140.0)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.number_freq = value;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                NumberInput::new("num-db")
                                    .value(number_db)
                                    .min(-12.0)
                                    .max(12.0)
                                    .step(0.5)
                                    .decimals(1)
                                    .unit("dB")
                                    .label("Gain")
                                    .size(NumberInputSize::Sm)
                                    .width(100.0)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.number_db = value;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                NumberInput::new("num-disabled")
                                    .value(50.0)
                                    .disabled(true)
                                    .label("Disabled")
                                    .size(NumberInputSize::Md)
                                    .width(100.0),
                            ),
                    ),
            )
            // Vertical Slider
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Vertical Slider").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::End)
                            .child(
                                VerticalSlider::new("vslider-sm")
                                    .value(vertical_slider_value)
                                    .min(0.0)
                                    .max(1.0)
                                    .label("Vol")
                                    .unit("%")
                                    .size(VerticalSliderSize::Sm)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.vertical_slider_value = value;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("vslider-md")
                                    .value(vertical_slider_value)
                                    .min(-12.0)
                                    .max(12.0)
                                    .label("Gain")
                                    .unit("dB")
                                    .size(VerticalSliderSize::Md)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _cx| {
                                                showcase.vertical_slider_value = value;
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("vslider-lg")
                                    .value(vertical_slider_value)
                                    .min(0.0)
                                    .max(100.0)
                                    .label("Level")
                                    .size(VerticalSliderSize::Lg)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.vertical_slider_value = value;
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            // Text Input (NEW: Full keyboard editing!)
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Text::new("✨ Text Input (NEW: Full keyboard editing!)")
                            .weight(TextWeight::Medium),
                    )
                    .child(
                        Text::new(
                            "Try: Click to edit, type text, Enter to confirm, Escape to cancel",
                        )
                        .size(TextSize::Xs)
                        .muted(true),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child({
                                let entity = entity.clone();
                                Input::new("input-editable")
                                    .label("Editable Text")
                                    .value(input_value.clone())
                                    .placeholder("Click to edit...")
                                    .variant(InputVariant::Default)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |new_value, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.input_value = new_value.to_string();
                                            });
                                        }
                                    })
                                    .on_text_change({
                                        move |text, _window, cx| {
                                            entity.update(cx, |showcase, _| {
                                                showcase.input_edit_text = text;
                                            });
                                        }
                                    })
                            })
                            .child(
                                Input::new("input-filled")
                                    .placeholder("Filled variant...")
                                    .variant(InputVariant::Filled),
                            )
                            .child(
                                Input::new("input-disabled")
                                    .value("Disabled")
                                    .disabled(true),
                            ),
                    ),
            )
            // Select with keyboard navigation
            .child({
                let select_value = self.select_value.clone();
                let select_open = self.select_open;
                let select_highlighted = self.select_highlighted;
                let entity_sel = entity.clone();

                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Text::new("Select Dropdown (✨ NEW: Arrow keys, Enter, Escape, Space!)")
                            .weight(TextWeight::Medium),
                    )
                    .child(
                        Text::new(
                            "Try: Space to open, ↑↓ to navigate, Enter to select, Esc to close",
                        )
                        .size(TextSize::Xs)
                        .muted(true),
                    )
                    .child(
                        div().w(px(200.0)).child(
                            Select::new("select-demo")
                                .options(vec![
                                    SelectOption::new("apple", "🍎 Apple"),
                                    SelectOption::new("banana", "🍌 Banana"),
                                    SelectOption::new("cherry", "🍒 Cherry"),
                                    SelectOption::new("grape", "🍇 Grape"),
                                    SelectOption::new("orange", "🍊 Orange"),
                                ])
                                .selected(select_value.unwrap_or("apple".into()))
                                .placeholder("Choose a fruit...")
                                .label("Fruit Selection")
                                .is_open(select_open)
                                .highlighted_index(select_highlighted)
                                .on_change({
                                    let entity = entity_sel.clone();
                                    move |value, _window, cx| {
                                        entity.update(cx, |showcase, _cx| {
                                            showcase.select_value = Some(value.clone());
                                            showcase.select_open = false;
                                            showcase.select_highlighted = None;
                                        });
                                    }
                                })
                                .on_toggle({
                                    let entity = entity_sel.clone();
                                    move |open, _window, cx| {
                                        entity.update(cx, |showcase, _cx| {
                                            showcase.select_open = open;
                                        });
                                    }
                                })
                                .on_highlight({
                                    let entity = entity_sel.clone();
                                    move |idx, _window, cx| {
                                        entity.update(cx, |showcase, _cx| {
                                            showcase.select_highlighted = idx;
                                        });
                                    }
                                }),
                        ),
                    )
            })
            // ButtonSet (segmented control)
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Button Set (Segmented Control)").weight(TextWeight::Medium))
                    .child(
                        Text::new("Mutually exclusive button group - click to select")
                            .size(TextSize::Xs)
                            .muted(true),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::Start)
                            // View mode example
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("View Mode:").size(TextSize::Xs).muted(true))
                                    .child(
                                        ButtonSet::new("view-mode")
                                            .options(vec![
                                                ButtonSetOption::new("list", "List"),
                                                ButtonSetOption::new("grid", "Grid"),
                                                ButtonSetOption::new("table", "Table"),
                                            ])
                                            .selected(buttonset_view_mode)
                                            .size(ButtonSetSize::Md)
                                            .on_change({
                                                let entity = entity.clone();
                                                move |value, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        showcase.buttonset_view_mode = value.clone();
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Alignment example (small size)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Alignment (Sm):").size(TextSize::Xs).muted(true))
                                    .child(
                                        ButtonSet::new("alignment")
                                            .options(vec![
                                                ButtonSetOption::new("left", "L"),
                                                ButtonSetOption::new("center", "C"),
                                                ButtonSetOption::new("right", "R"),
                                            ])
                                            .selected(buttonset_alignment)
                                            .size(ButtonSetSize::Sm)
                                            .on_change({
                                                let entity = entity.clone();
                                                move |value, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        showcase.buttonset_alignment = value.clone();
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Large size example (static)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Toggle (Lg):").size(TextSize::Xs).muted(true))
                                    .child(
                                        ButtonSet::new("toggle-lg")
                                            .options(vec![
                                                ButtonSetOption::new("on", "ON"),
                                                ButtonSetOption::new("off", "OFF"),
                                            ])
                                            .selected("on")
                                            .size(ButtonSetSize::Lg),
                                    ),
                            )
                            // Disabled option example
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("With disabled:").size(TextSize::Xs).muted(true))
                                    .child(
                                        ButtonSet::new("disabled-demo")
                                            .options(vec![
                                                ButtonSetOption::new("a", "A"),
                                                ButtonSetOption::new("b", "B").disabled(true),
                                                ButtonSetOption::new("c", "C"),
                                            ])
                                            .selected("a")
                                            .size(ButtonSetSize::Md),
                                    ),
                            ),
                    ),
            )
            // Keyboard support summary
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Text::new("Keyboard Support Summary")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Lg),
                    )
                    .child(Divider::new().build())
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Checkbox").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• Space/Enter: Toggle")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Text Input ✨").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• Click to edit text")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• Type, Backspace to edit")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• Enter/Esc: Save/Cancel")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Number Input").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• Click value to edit")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• ↑↓: Inc/Dec, Enter/Esc")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Select").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• Space: Toggle open/close")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• ↑↓: Navigate options")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• Enter: Select, Esc: Close")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            ),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Volume Knob").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• ↑↓/←→: Adjust volume")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• M: Toggle mute")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Sliders").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• ↑↓/←→: Adjust value")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• Home/End: Min/Max")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Potentiometer").weight(TextWeight::Medium))
                                    .child(
                                        Text::new("• ↑↓/←→: Adjust value")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    )
                                    .child(
                                        Text::new("• Esc: Reset to default")
                                            .size(TextSize::Xs)
                                            .muted(true),
                                    ),
                            ),
                    ),
            )
    }
}
