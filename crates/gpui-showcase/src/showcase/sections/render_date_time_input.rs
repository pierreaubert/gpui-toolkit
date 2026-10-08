use super::prelude::*;
use gpui_ui_kit::date_picker::CalendarDate;

impl Showcase {
    pub(crate) fn render_date_time_input_section(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionDateTimeInput);
        let date = CalendarDate {
            year: 2026,
            month: 10,
            day: 7,
        };
        let time = ClockTime::new(9, 30).expect("fixed showcase time is valid");

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Populated parts
            .child(Text::new("Date and time:").weight(TextWeight::Semibold))
            .child(DateTimeInput::new("datetime-filled").date(date).time(time))
            // Empty and disabled states
            .child(Text::new("Empty, clearable, and disabled:").weight(TextWeight::Semibold))
            .child(DateTimeInput::new("datetime-empty"))
            .child(
                DateTimeInput::new("datetime-clearable")
                    .date(date)
                    .time(time)
                    .clearable(true),
            )
            .child(
                DateTimeInput::new("datetime-disabled")
                    .date(date)
                    .time(time)
                    .disabled(true),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                DateTimeInput::new("datetime-sm")
                    .size(DateTimeInputSize::Sm)
                    .date(date)
                    .time(time),
            )
            .child(
                DateTimeInput::new("datetime-lg")
                    .size(DateTimeInputSize::Lg)
                    .date(date)
                    .time(time),
            )
    }
}
