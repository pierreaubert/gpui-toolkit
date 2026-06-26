use super::prelude::*;

impl Showcase {
    pub(crate) fn render_qr_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionQrCode);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Static QR codes
            .child(Text::new("Static QR codes (size large enough for all modules):").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .align(StackAlign::End)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Default (200px)").weight(TextWeight::Medium))
                            .child(QrCode::new("https://example.com")),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Small (120px)").weight(TextWeight::Medium))
                            .child(QrCode::new("https://example.com").size(px(120.0))),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Custom Colors").weight(TextWeight::Medium))
                            .child(
                                QrCode::new("https://example.com")
                                    .size(px(150.0))
                                    .fg(rgba(0x2da44eff))
                                    .bg(rgba(0x1a1a2eff)),
                            ),
                    ),
            )
            // Animated QR codes
            .child(
                Text::new("Animated QR codes (size too small for modules — auto-pans a zoomed viewport):")
                    .weight(TextWeight::Semibold),
            )
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .align(StackAlign::End)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Tiny (50px)").weight(TextWeight::Medium))
                            .child(self.animated_qr_tiny.clone()),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Small (80px)").weight(TextWeight::Medium))
                            .child(self.animated_qr_small.clone()),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                Text::new("Normal (200px) — no animation needed")
                                    .weight(TextWeight::Medium),
                            )
                            .child(Text::new("At larger sizes, AnimatedQrCode renders identically to QrCode.").muted(true))
                            .child(QrCode::new("https://example.com/animated-qr-demo").size(px(200.0))),
                    ),
            )
    }
}
