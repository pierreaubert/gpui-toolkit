use super::prelude::*;

impl Showcase {
    pub(crate) fn render_calendar_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionCalendar);
        let entity = self.weak_entity_handle();
        let (year, month) = (self.calendar_year, self.calendar_month);
        let selected = self.calendar_selected;

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive month
            .child(Text::new("Pick a day:").weight(TextWeight::Semibold))
            .child({
                let entity_clone = entity.clone();
                let navigate_entity = entity.clone();
                let mut calendar = Calendar::new("showcase-calendar").visible(year, month);
                if let Some(date) = selected {
                    calendar = calendar.selected(date);
                }
                calendar
                    .on_select(move |date, _window, cx| {
                        entity_clone.update(cx, |this, cx| {
                            this.calendar_selected = Some(date);
                            this.calendar_year = date.year;
                            this.calendar_month = date.month;
                            this.notify_content(cx);
                        });
                    })
                    .on_navigate(move |year, month, _window, cx| {
                        navigate_entity.update(cx, |this, cx| {
                            this.calendar_year = year;
                            this.calendar_month = month;
                            this.notify_content(cx);
                        });
                    })
            })
            // Bounded range
            .child(Text::new("Bounded range:").weight(TextWeight::Semibold))
            .child(
                Calendar::new("showcase-calendar-bounded")
                    .visible(2026, 10)
                    .min(CalendarDate::new(2026, 10, 5).unwrap())
                    .max(CalendarDate::new(2026, 10, 20).unwrap())
                    .size(CalendarSize::Sm),
            )
    }
}
