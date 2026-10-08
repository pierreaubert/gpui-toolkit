use super::prelude::*;
use gpui_ui_kit::date_picker::CalendarDate;

impl Showcase {
    pub(crate) fn render_date_range_input_section(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionDateRangeInput);
        let from = CalendarDate {
            year: 2026,
            month: 10,
            day: 1,
        };
        let to = CalendarDate {
            year: 2026,
            month: 10,
            day: 7,
        };

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Populated range
            .child(Text::new("Range with bounds:").weight(TextWeight::Semibold))
            .child(DateRangeInput::new("range-filled").start(from).end(to))
            // Empty and disabled states
            .child(Text::new("Empty, clearable, and disabled:").weight(TextWeight::Semibold))
            .child(DateRangeInput::new("range-empty"))
            .child(
                DateRangeInput::new("range-clearable")
                    .start(from)
                    .end(to)
                    .clearable(true),
            )
            .child(
                DateRangeInput::new("range-disabled")
                    .start(from)
                    .disabled(true),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                DateRangeInput::new("range-sm")
                    .size(DateRangeInputSize::Sm)
                    .start(from)
                    .end(to),
            )
            .child(
                DateRangeInput::new("range-lg")
                    .size(DateRangeInputSize::Lg)
                    .start(from)
                    .end(to),
            )
    }
}
