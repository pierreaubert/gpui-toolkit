impl Showcase {
    fn render_breadcrumbs_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionBreadcrumbs);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Different separators
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Breadcrumbs::new()
                            .separator(BreadcrumbSeparator::Slash)
                            .items(vec![
                                BreadcrumbItem::new("home", "Home").icon("H"),
                                BreadcrumbItem::new("products", "Products"),
                                BreadcrumbItem::new("category", "Electronics"),
                                BreadcrumbItem::new("item", "Laptop"),
                            ]),
                    )
                    .child(
                        Breadcrumbs::new()
                            .separator(BreadcrumbSeparator::Chevron)
                            .items(vec![
                                BreadcrumbItem::new("root", "Root"),
                                BreadcrumbItem::new("folder", "Folder"),
                                BreadcrumbItem::new("file", "File.txt"),
                            ]),
                    )
                    .child(
                        Breadcrumbs::new()
                            .separator(BreadcrumbSeparator::Dot)
                            .items(vec![
                                BreadcrumbItem::new("app", "App"),
                                BreadcrumbItem::new("settings", "Settings"),
                                BreadcrumbItem::new("account", "Account"),
                            ]),
                    ),
            )
    }
}
