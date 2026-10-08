use super::prelude::*;

impl Showcase {
    pub(crate) fn render_app_shell_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionAppShell);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Full slot composition
            .child(Text::new("Header, sidebar, content, and footer:").weight(TextWeight::Semibold))
            .child(
                div().h(px(320.0)).child(
                    AppShell::new("shell-full")
                        .header(
                            TopNav::new("shell-nav")
                                .brand(Text::new("Demo").weight(TextWeight::Semibold))
                                .item(TopNavItem::new("home", "Home").active(true))
                                .item(TopNavItem::new("docs", "Docs")),
                        )
                        .sidebar(
                            Sidebar::new("shell-side")
                                .header(Text::new("Filters").weight(TextWeight::Semibold))
                                .content(Text::new("Sidebar content")),
                        )
                        .content(
                            div()
                                .p_4()
                                .child(Text::new("Main content fills the remaining space.")),
                        )
                        .footer(StatusBar::new("shell-status").center(Text::new("Ready"))),
                ),
            )
            // Right sidebar without a header
            .child(Text::new("Right sidebar, no header:").weight(TextWeight::Semibold))
            .child(
                div().h(px(220.0)).child(
                    AppShell::new("shell-right")
                        .sidebar_side(AppShellSidebarSide::Right)
                        .sidebar(Text::new("Tools"))
                        .content(
                            div()
                                .p_4()
                                .child(Text::new("Content with a right-side panel.")),
                        ),
                ),
            )
    }
}
