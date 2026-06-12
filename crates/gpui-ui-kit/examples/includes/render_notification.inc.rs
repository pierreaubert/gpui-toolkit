impl Showcase {
    fn render_notification_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionNotification);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Info
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        Notification::new("notif-info", "New update available")
                            .variant(NotificationVariant::Info)
                            .description("Version 2.0 is ready to install."),
                    ),
            )
            // Success
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        Notification::new("notif-success", "File saved")
                            .variant(NotificationVariant::Success),
                    ),
            )
            // Warning
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        Notification::new("notif-warning", "Low disk space")
                            .variant(NotificationVariant::Warning)
                            .description("Only 2 GB remaining on your drive."),
                    ),
            )
            // Error with action
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        Notification::new("notif-error", "Connection lost")
                            .variant(NotificationVariant::Error)
                            .description("Unable to reach the server. Check your network.")
                            .action("Retry", |_window, _cx| {}),
                    ),
            )
    }
}
