use super::prelude::*;

impl Showcase {
    pub(crate) fn render_time_input_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTimeInput);
        let morning = ClockTime::new(9, 30).expect("fixed showcase time is valid");

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Populated value
            .child(Text::new("Time with value:").weight(TextWeight::Semibold))
            .child(TimeInput::new("time-filled").value(morning))
            // Empty and disabled states
            .child(Text::new("Empty, clearable, and disabled:").weight(TextWeight::Semibold))
            .child(TimeInput::new("time-empty"))
            .child(
                TimeInput::new("time-clearable")
                    .value(morning)
                    .clearable(true),
            )
            .child(
                TimeInput::new("time-disabled")
                    .value(morning)
                    .disabled(true),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                TimeInput::new("time-sm")
                    .size(TimeInputSize::Sm)
                    .value(morning),
            )
            .child(
                TimeInput::new("time-lg")
                    .size(TimeInputSize::Lg)
                    .value(morning),
            )
    }
}
