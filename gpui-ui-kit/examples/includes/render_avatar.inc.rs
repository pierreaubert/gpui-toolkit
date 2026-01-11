impl Showcase {
    fn render_avatars_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionAvatars);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Avatar Sizes
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .align(StackAlign::End)
                    .child(Avatar::new().name("John Doe").size(AvatarSize::Xs))
                    .child(Avatar::new().name("Jane Smith").size(AvatarSize::Sm))
                    .child(Avatar::new().name("Bob Wilson").size(AvatarSize::Md))
                    .child(Avatar::new().name("Alice Brown").size(AvatarSize::Lg))
                    .child(Avatar::new().name("Charlie Davis").size(AvatarSize::Xl)),
            )
            // Avatar Shapes and Status
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Avatar::new().name("Circle").shape(AvatarShape::Circle))
                    .child(Avatar::new().name("Square").shape(AvatarShape::Square))
                    .child(Avatar::new().name("Online").status(AvatarStatus::Online))
                    .child(Avatar::new().name("Away").status(AvatarStatus::Away))
                    .child(Avatar::new().name("Busy").status(AvatarStatus::Busy))
                    .child(Avatar::new().name("Offline").status(AvatarStatus::Offline)),
            )
            // Avatar Group
            .child(
                AvatarGroup::new()
                    .avatars(vec![
                        Avatar::new().name("User One"),
                        Avatar::new().name("User Two"),
                        Avatar::new().name("User Three"),
                        Avatar::new().name("User Four"),
                        Avatar::new().name("User Five"),
                        Avatar::new().name("User Six"),
                    ])
                    .max_display(4)
                    .size(AvatarSize::Md),
            )
    }
}
