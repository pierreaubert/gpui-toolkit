//! `Chat` component
//!
//! A conversation transcript: an ordered list of author-labeled message
//! bubbles. Messages are parent-owned data; the transcript itself offers
//! no composer or scrolling, only role-based bubble styling.
//!
//! # Usage
//!
//! ```ignore
//! Chat::new("support-thread")
//!     .message(ChatMessage::user("Ada", "The build failed."))
//!     .message(ChatMessage::assistant("It looks like a missing key."))
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Speaker role for one chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatRole {
    /// Human participant; bubble aligns trailing.
    #[default]
    User,
    /// Machine participant; bubble aligns leading.
    Assistant,
    /// Centered status line without a bubble.
    System,
}

/// One transcript entry with author, body, and role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// Display name shown above the bubble.
    pub author: SharedString,
    /// Message body text.
    pub body: SharedString,
    /// Speaker role controlling alignment and styling.
    pub role: ChatRole,
}

impl ChatMessage {
    /// Create a user message from `author` with `body`.
    pub fn user(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
            role: ChatRole::User,
        }
    }

    /// Create an assistant message with `body`.
    pub fn assistant(body: impl Into<SharedString>) -> Self {
        Self {
            author: SharedString::from("Assistant"),
            body: body.into(),
            role: ChatRole::Assistant,
        }
    }

    /// Create a centered system status line with `body`.
    pub fn system(body: impl Into<SharedString>) -> Self {
        Self {
            author: SharedString::from("System"),
            body: body.into(),
            role: ChatRole::System,
        }
    }
}

/// Theme colors for the chat transcript.
#[derive(Debug, Clone, ComponentTheme)]
pub struct ChatTheme {
    /// User bubble background.
    #[theme(default = 0x007accff, from = accent)]
    pub user_bubble: Rgba,
    /// User bubble text.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub user_text: Rgba,
    /// Assistant bubble background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub assistant_bubble: Rgba,
    /// Assistant bubble text.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub assistant_text: Rgba,
    /// Author caption and system line color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub caption: Rgba,
}

/// Chat transcript size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatSize {
    /// Compact bubbles.
    Sm,
    /// Standard bubbles (default).
    #[default]
    Md,
    /// Large bubbles.
    Lg,
}

impl From<crate::ComponentSize> for ChatSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Conversation transcript with role-based bubbles.
#[derive(IntoElement)]
pub struct Chat {
    id: ElementId,
    messages: Vec<ChatMessage>,
    size: ChatSize,
    theme: Option<ChatTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Chat {
    /// Create a new empty transcript.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            messages: Vec::new(),
            size: ChatSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the full message list.
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Append one message to the transcript.
    pub fn message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Set the transcript size.
    pub fn size(mut self, size: ChatSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: ChatTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Group).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Bubble colors for one role.
    fn bubble_colors(role: ChatRole, theme: &ChatTheme) -> (Rgba, Rgba) {
        match role {
            ChatRole::User => (theme.user_bubble, theme.user_text),
            ChatRole::Assistant | ChatRole::System => {
                (theme.assistant_bubble, theme.assistant_text)
            }
        }
    }

    /// Build the transcript with theme.
    pub fn build_with_theme(self, theme: &ChatTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the transcript with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &ChatTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut transcript = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap));
        transcript = match self.size {
            ChatSize::Sm => transcript.text_xs(),
            ChatSize::Md => transcript.text_sm(),
            ChatSize::Lg => transcript.text_lg(),
        };

        for message in self.messages {
            if message.role == ChatRole::System {
                transcript = transcript.child(
                    div()
                        .w_full()
                        .text_center()
                        .text_color(theme.caption)
                        .child(message.body),
                );
                continue;
            }
            let (bubble, text) = Self::bubble_colors(message.role, theme);
            let mut row = div().flex().flex_col().w_full();
            row = match message.role {
                ChatRole::User => row.items_end(),
                _ => row.items_start(),
            };
            row = row
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.caption)
                        .child(message.author),
                )
                .child(
                    div()
                        .px(px(design.spacing.control_padding_x))
                        .py(px(design.spacing.control_padding_y * 0.5))
                        .rounded(px(design.corners.md))
                        .bg(bubble)
                        .text_color(text)
                        .child(message.body),
                );
            transcript = transcript.child(row);
        }

        transcript
    }
}

impl RenderOnce for Chat {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| "Conversation".into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| ChatTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Chat, ChatMessage, ChatRole, ChatSize, ChatTheme};
    use gpui::rgba;

    #[test]
    fn chat_defaults_to_empty_standard_transcript() {
        let chat = Chat::new("support-thread");
        assert!(chat.messages.is_empty());
        assert_eq!(chat.size, ChatSize::Md);
    }

    #[test]
    fn chat_message_constructors_set_roles() {
        let user = ChatMessage::user("Ada", "Hi");
        assert_eq!(user.role, ChatRole::User);
        assert_eq!(user.author.as_ref(), "Ada");
        let assistant = ChatMessage::assistant("Hello");
        assert_eq!(assistant.role, ChatRole::Assistant);
        let system = ChatMessage::system("Joined");
        assert_eq!(system.role, ChatRole::System);
    }

    #[test]
    fn chat_builders_store_messages_and_size() {
        let chat = Chat::new("support-thread")
            .message(ChatMessage::user("Ada", "Hi"))
            .messages(vec![ChatMessage::system("Joined")])
            .size(ChatSize::Lg);
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, ChatRole::System);
        assert_eq!(chat.size, ChatSize::Lg);
    }

    #[test]
    fn chat_bubble_colors_follow_roles() {
        let theme = ChatTheme::default();
        assert_eq!(
            Chat::bubble_colors(ChatRole::User, &theme),
            (theme.user_bubble, theme.user_text)
        );
        assert_eq!(
            Chat::bubble_colors(ChatRole::Assistant, &theme),
            (theme.assistant_bubble, theme.assistant_text)
        );
    }

    #[test]
    fn chat_theme_defaults_match_compiled_tokens() {
        let theme = ChatTheme::default();
        assert_eq!(theme.user_bubble, rgba(0x007accff));
        assert_eq!(theme.user_text, rgba(0xffffffff));
        assert_eq!(theme.assistant_bubble, rgba(0x1e1e1eff));
        assert_eq!(theme.assistant_text, rgba(0xccccccff));
        assert_eq!(theme.caption, rgba(0x777777ff));
    }
}
