impl Showcase {
    fn render_menu_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionMenus);
        let file = cx.t(TranslationKey::MenuFile);
        let edit = cx.t(TranslationKey::MenuEdit);
        let view = cx.t(TranslationKey::MenuView);
        let help = cx.t(TranslationKey::MenuHelp);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Menu component
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Dropdown Menu").weight(TextWeight::Medium))
                    .child(
                        Menu::new("example-menu", vec![
                            MenuItem::new("new-file", "New File")
                                .with_shortcut("Cmd+N")
                                .with_icon("N"),
                            MenuItem::new("open", "Open...")
                                .with_shortcut("Cmd+O")
                                .with_icon("O"),
                            MenuItem::new("save", "Save")
                                .with_shortcut("Cmd+S")
                                .with_icon("S"),
                            MenuItem::separator(),
                            MenuItem::checkbox("autosave", "Auto Save", true),
                            MenuItem::separator(),
                            MenuItem::new("quit", "Quit")
                                .with_shortcut("Cmd+Q")
                                .danger(),
                        ])
                        .min_width(px(220.0)),
                    ),
            )
            // Menu Bar info
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Menu Bar Items").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(menu_bar_button("file", file, false, &MenuTheme::default()))
                            .child(menu_bar_button("edit", edit, false, &MenuTheme::default()))
                            .child(menu_bar_button("view", view, true, &MenuTheme::default()))
                            .child(menu_bar_button("help", help, false, &MenuTheme::default())),
                    ),
            )
    }
}
