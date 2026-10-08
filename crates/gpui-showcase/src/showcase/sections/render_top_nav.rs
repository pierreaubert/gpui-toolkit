use super::prelude::*;

impl Showcase {
    pub(crate) fn render_top_nav_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTopNav);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Basic bar
            .child(Text::new("Brand, items, and trailing actions:").weight(TextWeight::Semibold))
            .child(
                TopNav::new("topnav-basic")
                    .brand(Text::new("Acme").weight(TextWeight::Semibold))
                    .item(TopNavItem::new("home", "Home").active(true))
                    .item(TopNavItem::new("products", "Products"))
                    .item(TopNavItem::new("pricing", "Pricing").disabled(true))
                    .trailing(Button::new("topnav-cta", "Sign in")),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                TopNav::new("topnav-sm")
                    .size(TopNavSize::Sm)
                    .bordered(false)
                    .item(TopNavItem::new("sm-home", "Home").active(true))
                    .item(TopNavItem::new("sm-docs", "Docs")),
            )
            .child(
                TopNav::new("topnav-lg")
                    .size(TopNavSize::Lg)
                    .item(TopNavItem::new("lg-home", "Home").active(true))
                    .item(TopNavItem::new("lg-docs", "Docs")),
            )
    }
}
