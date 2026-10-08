use super::prelude::*;

impl Showcase {
    pub(crate) fn render_chat_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionChat);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Role styling
            .child(Text::new("User, assistant, and system:").weight(TextWeight::Semibold))
            .child(
                Chat::new("chat-roles")
                    .message(ChatMessage::system("Ada joined the thread"))
                    .message(ChatMessage::user("Ada", "The nightly build failed."))
                    .message(ChatMessage::assistant(
                        "It looks like a missing signing key. I queued a retry.",
                    )),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                Chat::new("chat-sm")
                    .size(ChatSize::Sm)
                    .message(ChatMessage::user("Ada", "Compact transcript."))
                    .message(ChatMessage::assistant("Compact reply.")),
            )
            .child(
                Chat::new("chat-lg")
                    .size(ChatSize::Lg)
                    .message(ChatMessage::user("Ada", "Large transcript."))
                    .message(ChatMessage::assistant("Large reply.")),
            )
    }
}
