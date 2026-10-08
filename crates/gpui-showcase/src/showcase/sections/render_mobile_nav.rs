use super::prelude::*;

impl Showcase {
    pub(crate) fn render_mobile_nav_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionMobileNav);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Basic tab bar
            .child(Text::new("Tabs with parent-owned selection:").weight(TextWeight::Semibold))
            .child(
                MobileNav::new("mobilenav-basic")
                    .item(MobileNavItem::new("home", "Home").icon("⌂"))
                    .item(MobileNavItem::new("search", "Search").icon("⌕"))
                    .item(MobileNavItem::new("library", "Library").icon("▤"))
                    .selected("search"),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                MobileNav::new("mobilenav-sm")
                    .size(MobileNavSize::Sm)
                    .item(MobileNavItem::new("sm-home", "Home"))
                    .item(MobileNavItem::new("sm-search", "Search"))
                    .selected("sm-home"),
            )
            .child(
                MobileNav::new("mobilenav-lg")
                    .size(MobileNavSize::Lg)
                    .item(MobileNavItem::new("lg-home", "Home").icon("⌂"))
                    .item(MobileNavItem::new("lg-search", "Search").icon("⌕"))
                    .selected("lg-search"),
            )
    }
}
