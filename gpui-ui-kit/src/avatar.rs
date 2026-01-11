//! Avatar component
//!
//! User avatars and profile images.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::*;

/// Avatar size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarSize {
    /// Extra small (20px)
    Xs,
    /// Small (24px)
    Sm,
    /// Medium (32px, default)
    #[default]
    Md,
    /// Large (40px)
    Lg,
    /// Extra large (48px)
    Xl,
    /// 2X large (64px)
    Xxl,
}

impl AvatarSize {
    fn size(&self) -> Pixels {
        match self {
            AvatarSize::Xs => px(20.0),
            AvatarSize::Sm => px(24.0),
            AvatarSize::Md => px(32.0),
            AvatarSize::Lg => px(40.0),
            AvatarSize::Xl => px(48.0),
            AvatarSize::Xxl => px(64.0),
        }
    }
}

impl From<crate::ComponentSize> for AvatarSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg => Self::Lg,
            crate::ComponentSize::Xl => Self::Xl,
        }
    }
}

/// Avatar shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarShape {
    /// Circular (default)
    #[default]
    Circle,
    /// Rounded square
    Square,
}

/// An avatar component
pub struct Avatar {
    name: Option<SharedString>,
    src: Option<SharedString>,
    size: AvatarSize,
    shape: AvatarShape,
    status: Option<AvatarStatus>,
}

/// Avatar online status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarStatus {
    Online,
    Offline,
    Away,
    Busy,
}

impl AvatarStatus {
    fn color(&self, theme: &Theme) -> Rgba {
        match self {
            AvatarStatus::Online => theme.success,
            AvatarStatus::Offline => theme.text_muted,
            AvatarStatus::Away => theme.warning,
            AvatarStatus::Busy => theme.error,
        }
    }
}

impl Avatar {
    /// Create a new avatar
    pub fn new() -> Self {
        Self {
            name: None,
            src: None,
            size: AvatarSize::default(),
            shape: AvatarShape::default(),
            status: None,
        }
    }

    /// Set name (used for initials fallback)
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set image source
    pub fn src(mut self, src: impl Into<SharedString>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Set size
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Set shape
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set status indicator
    pub fn status(mut self, status: AvatarStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Get initials from name
    fn get_initials(&self) -> String {
        if let Some(name) = &self.name {
            name.split_whitespace()
                .filter_map(|word| word.chars().next())
                .take(2)
                .collect::<String>()
                .to_uppercase()
        } else {
            "?".to_string()
        }
    }

    /// Get background color based on name hash
    fn get_bg_color(&self) -> Rgba {
        if let Some(name) = &self.name {
            let hash: u32 = name.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
            let colors = [
                rgb(0x007acc), // Blue
                rgb(0x2da44e), // Green
                rgb(0xd29922), // Yellow
                rgb(0xcc3333), // Red
                rgb(0x8b5cf6), // Purple
                rgb(0x06b6d4), // Cyan
                rgb(0xf97316), // Orange
                rgb(0xec4899), // Pink
            ];
            colors[(hash as usize) % colors.len()]
        } else {
            rgb(0x3a3a3a)
        }
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let size = self.size.size();
        let initials = self.get_initials();
        let bg_color = self.get_bg_color();

        let mut avatar = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .w(size)
            .h(size)
            .bg(bg_color)
            .text_color(theme.text_primary)
            .overflow_hidden();

        // Apply shape
        match self.shape {
            AvatarShape::Circle => {
                avatar = avatar.rounded_full();
            }
            AvatarShape::Square => {
                avatar = avatar.rounded_md();
            }
        }

        // Apply text size based on avatar size
        avatar = match self.size {
            AvatarSize::Xs => avatar.text_xs(),
            AvatarSize::Sm => avatar.text_xs(),
            AvatarSize::Md => avatar.text_sm(),
            AvatarSize::Lg => avatar.text_sm(),
            AvatarSize::Xl => avatar,
            AvatarSize::Xxl => avatar.text_lg(),
        };

        // Content: image or initials
        if let Some(_src) = self.src {
            // Note: Image loading requires gpui::img()
            // For now, show initials as fallback
            avatar = avatar.child(initials);
        } else {
            avatar = avatar.font_weight(FontWeight::SEMIBOLD).child(initials);
        }

        // Status indicator
        if let Some(status) = self.status {
            let status_size = match self.size {
                AvatarSize::Xs | AvatarSize::Sm => px(6.0),
                AvatarSize::Md | AvatarSize::Lg => px(8.0),
                AvatarSize::Xl | AvatarSize::Xxl => px(10.0),
            };

            let status_indicator = div()
                .absolute()
                .bottom_0()
                .right_0()
                .w(status_size)
                .h(status_size)
                .rounded_full()
                .bg(status.color(theme))
                .border_2()
                .border_color(theme.background);

            avatar = avatar.child(status_indicator);
        }

        avatar
    }
}

impl Default for Avatar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Avatar {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A group of avatars displayed overlapping
pub struct AvatarGroup {
    avatars: Vec<Avatar>,
    max_display: usize,
    size: AvatarSize,
}

impl AvatarGroup {
    /// Create a new avatar group
    pub fn new() -> Self {
        Self {
            avatars: Vec::new(),
            max_display: 4,
            size: AvatarSize::default(),
        }
    }

    /// Add avatars
    pub fn avatars(mut self, avatars: Vec<Avatar>) -> Self {
        self.avatars = avatars;
        self
    }

    /// Set maximum number to display
    pub fn max_display(mut self, max: usize) -> Self {
        self.max_display = max;
        self
    }

    /// Set size for all avatars
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let size = self.size.size();
        let overlap = size * 0.3;

        let mut container = div().flex().items_center();

        let display_count = self.avatars.len().min(self.max_display);
        let remaining = self.avatars.len().saturating_sub(self.max_display);

        for (i, avatar) in self.avatars.into_iter().take(display_count).enumerate() {
            let avatar_el = avatar.size(self.size).build_with_theme(theme);
            let mut wrapper = div().relative();

            if i > 0 {
                wrapper = wrapper.ml(-overlap);
            }

            wrapper = wrapper.child(avatar_el);
            container = container.child(wrapper);
        }

        // Show remaining count
        if remaining > 0 {
            container = container.child(
                div()
                    .ml(-overlap)
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(size)
                    .h(size)
                    .rounded_full()
                    .bg(theme.surface)
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("+{}", remaining)),
            );
        }

        container
    }
}

impl Default for AvatarGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for AvatarGroup {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
