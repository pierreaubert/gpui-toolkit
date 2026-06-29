use super::prelude::*;

impl Showcase {
    pub(crate) fn render_settings_form_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSettingsForm);
        let theme = cx.theme();
        let entity = self.entity.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        SettingsForm::new("demo-settings")
                            .label_width(px(200.0))
                            .section("Playback")
                            .row(
                                SettingsRow::new("Volume")
                                    .description("Master output volume")
                                    .control(Text::new("75%").size(TextSize::Sm)),
                            )
                            .row(
                                SettingsRow::new("Mute")
                                    .description("Mute all audio output")
                                    .control(
                                        Toggle::new("settings-mute")
                                            .checked(self.settings_mute)
                                            .on_change(move |checked, _window, cx| {
                                                entity.update(cx, |this, _cx| {
                                                    this.settings_mute = checked;
                                                });
                                            }),
                                    ),
                            )
                            .section("Display")
                            .row(
                                SettingsRow::new("Theme")
                                    .description("Choose light or dark theme")
                                    .control(Text::new("Dark").size(TextSize::Sm)),
                            )
                            .row(
                                SettingsRow::new("Language")
                                    .description("UI display language")
                                    .control(Text::new("English").size(TextSize::Sm)),
                            )
                            .section("Advanced")
                            .row(
                                SettingsRow::new("Sample Rate")
                                    .description("Audio output sample rate")
                                    .control(Text::new("48000 Hz").size(TextSize::Sm)),
                            ),
                    ),
            )
    }
}
