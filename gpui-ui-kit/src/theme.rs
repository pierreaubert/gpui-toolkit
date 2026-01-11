//! Theme system for gpui-ui-kit
//!
//! Provides a unified theming system with light and dark themes.
//!
//! # Color Token Integration
//!
//! The theme system integrates with [`ColorToken`](crate::color_tokens::ColorToken)
//! for automatic generation of hover, active, muted, and subtle color variants:
//!
//! ```ignore
//! let theme = cx.theme();
//!
//! // Get a ColorToken for the accent color with auto-generated variants
//! let accent = theme.accent_token();
//!
//! div()
//!     .bg(accent.base)
//!     .hover(|s| s.bg(accent.hover))
//!     .active(|s| s.bg(accent.active))
//! ```

use crate::color_tokens::{
    BackgroundColors, BorderColors, ColorPalette, ColorToken, SemanticColors, TextColors,
};
use gpui::*;

/// Available theme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeVariant {
    /// Dark theme (default)
    #[default]
    Dark,
    /// Light theme
    Light,
    /// Midnight theme (deep blue)
    Midnight,
    /// Forest theme (green tones)
    Forest,
    /// Black & White theme (monochrome high contrast)
    BlackAndWhite,
}

impl ThemeVariant {
    /// Get all available variants
    pub fn all() -> &'static [ThemeVariant] {
        &[
            ThemeVariant::Dark,
            ThemeVariant::Light,
            ThemeVariant::Midnight,
            ThemeVariant::Forest,
            ThemeVariant::BlackAndWhite,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ThemeVariant::Dark => "Dark",
            ThemeVariant::Light => "Light",
            ThemeVariant::Midnight => "Midnight",
            ThemeVariant::Forest => "Forest",
            ThemeVariant::BlackAndWhite => "Black & White",
        }
    }

    /// Toggle to next variant
    pub fn toggle(&self) -> Self {
        match self {
            ThemeVariant::Dark => ThemeVariant::Light,
            ThemeVariant::Light => ThemeVariant::Midnight,
            ThemeVariant::Midnight => ThemeVariant::Forest,
            ThemeVariant::Forest => ThemeVariant::BlackAndWhite,
            ThemeVariant::BlackAndWhite => ThemeVariant::Dark,
        }
    }
}

/// Global theme colors
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme variant
    pub variant: ThemeVariant,

    // Background colors
    /// Main background color
    pub background: Rgba,
    /// Elevated surface background (cards, dialogs)
    pub surface: Rgba,
    /// Surface on hover
    pub surface_hover: Rgba,
    /// Muted background for secondary elements
    pub muted: Rgba,
    /// Transparent color (for invisible backgrounds)
    pub transparent: Rgba,
    /// Overlay/backdrop color (semi-transparent for modals/dialogs)
    pub overlay_bg: Rgba,

    // Text colors
    /// Primary text color
    pub text_primary: Rgba,
    /// Secondary/muted text color
    pub text_secondary: Rgba,
    /// Disabled text color
    pub text_muted: Rgba,

    // Accent colors
    /// Primary accent color
    pub accent: Rgba,
    /// Accent on hover
    pub accent_hover: Rgba,
    /// Muted accent for backgrounds
    pub accent_muted: Rgba,

    // Semantic colors
    /// Success color
    pub success: Rgba,
    /// Warning color
    pub warning: Rgba,
    /// Error color
    pub error: Rgba,
    /// Info color
    pub info: Rgba,

    // Border colors
    /// Default border
    pub border: Rgba,
    /// Border on hover/focus
    pub border_hover: Rgba,

    // Badge colors
    /// Badge primary background
    pub badge_primary_bg: Rgba,
    /// Badge primary text
    pub badge_primary_text: Rgba,
    /// Badge success background
    pub badge_success_bg: Rgba,
    /// Badge success text
    pub badge_success_text: Rgba,
    /// Badge warning background
    pub badge_warning_bg: Rgba,
    /// Badge warning text
    pub badge_warning_text: Rgba,
    /// Badge error background
    pub badge_error_bg: Rgba,
    /// Badge error text
    pub badge_error_text: Rgba,
    /// Badge info background
    pub badge_info_bg: Rgba,
    /// Badge info text
    pub badge_info_text: Rgba,
}

impl Theme {
    /// Create a dark theme
    pub fn dark() -> Self {
        Self {
            variant: ThemeVariant::Dark,
            // Backgrounds
            background: rgb(0x1e1e1e),
            surface: rgb(0x2a2a2a),
            surface_hover: rgb(0x3a3a3a),
            muted: rgb(0x252525),
            transparent: rgba(0x00000000),
            overlay_bg: rgba(0x00000088),
            // Text
            text_primary: rgb(0xffffff),
            text_secondary: rgb(0xcccccc),
            text_muted: rgb(0x888888),
            // Accent
            accent: rgb(0x007acc),
            accent_hover: rgb(0x0098ff),
            accent_muted: rgba(0x007acc33),
            // Semantic
            success: rgb(0x22c55e),
            warning: rgb(0xf59e0b),
            error: rgb(0xef4444),
            info: rgb(0x3b82f6),
            // Border
            border: rgb(0x3a3a3a),
            border_hover: rgb(0x555555),
            // Badge colors (dark theme)
            badge_primary_bg: rgb(0x1a4a7a),
            badge_primary_text: rgb(0x7cc4ff),
            badge_success_bg: rgb(0x1a3a1a),
            badge_success_text: rgb(0x7ccc7c),
            badge_warning_bg: rgb(0x3a3a1a),
            badge_warning_text: rgb(0xcccc7c),
            badge_error_bg: rgb(0x3a1a1a),
            badge_error_text: rgb(0xcc7c7c),
            badge_info_bg: rgb(0x1a3a3a),
            badge_info_text: rgb(0x7ccccc),
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,
            // Backgrounds
            background: rgb(0xf5f5f5),
            surface: rgb(0xffffff),
            surface_hover: rgb(0xf0f0f0),
            muted: rgb(0xeeeeee),
            transparent: rgba(0x00000000),
            overlay_bg: rgba(0x00000088),
            // Text
            text_primary: rgb(0x1a1a1a),
            text_secondary: rgb(0x4a4a4a),
            text_muted: rgb(0x888888),
            // Accent
            accent: rgb(0x0066cc),
            accent_hover: rgb(0x0055aa),
            accent_muted: rgba(0x0066cc22),
            // Semantic
            success: rgb(0x16a34a),
            warning: rgb(0xd97706),
            error: rgb(0xdc2626),
            info: rgb(0x2563eb),
            // Border
            border: rgb(0xd4d4d4),
            border_hover: rgb(0xaaaaaa),
            // Badge colors (light theme)
            badge_primary_bg: rgb(0xdbeafe),
            badge_primary_text: rgb(0x1d4ed8),
            badge_success_bg: rgb(0xdcfce7),
            badge_success_text: rgb(0x16a34a),
            badge_warning_bg: rgb(0xfef3c7),
            badge_warning_text: rgb(0xd97706),
            badge_error_bg: rgb(0xfee2e2),
            badge_error_text: rgb(0xdc2626),
            badge_info_bg: rgb(0xe0f2fe),
            badge_info_text: rgb(0x0284c7),
        }
    }

    /// Create a midnight theme (deep blue)
    pub fn midnight() -> Self {
        Self {
            variant: ThemeVariant::Midnight,
            // Backgrounds
            background: rgb(0x0d1117),
            surface: rgb(0x21262d),
            surface_hover: rgb(0x30363d),
            muted: rgb(0x161b22),
            transparent: rgba(0x00000000),
            overlay_bg: rgba(0x00000088),
            // Text
            text_primary: rgb(0xc9d1d9),
            text_secondary: rgb(0x8b949e),
            text_muted: rgb(0x6e7681),
            // Accent
            accent: rgb(0x58a6ff),
            accent_hover: rgb(0x79b8ff),
            accent_muted: rgba(0x1f6feb33),
            // Semantic
            success: rgb(0x3fb950),
            warning: rgb(0xd29922),
            error: rgb(0xf85149),
            info: rgb(0x58a6ff),
            // Border
            border: rgb(0x30363d),
            border_hover: rgb(0x484f58),
            // Badge colors (dark variant)
            badge_primary_bg: rgb(0x1a4a7a),
            badge_primary_text: rgb(0x7cc4ff),
            badge_success_bg: rgb(0x1a3a1a),
            badge_success_text: rgb(0x7ccc7c),
            badge_warning_bg: rgb(0x3a3a1a),
            badge_warning_text: rgb(0xcccc7c),
            badge_error_bg: rgb(0x3a1a1a),
            badge_error_text: rgb(0xcc7c7c),
            badge_info_bg: rgb(0x1a3a3a),
            badge_info_text: rgb(0x7ccccc),
        }
    }

    /// Create a forest theme (green tones)
    pub fn forest() -> Self {
        Self {
            variant: ThemeVariant::Forest,
            // Backgrounds
            background: rgb(0x1a2418),
            surface: rgb(0x2a3627),
            surface_hover: rgb(0x3a4a35),
            muted: rgb(0x222d1f),
            transparent: rgba(0x00000000),
            overlay_bg: rgba(0x00000088),
            // Text
            text_primary: rgb(0xd4e4d1),
            text_secondary: rgb(0xa8c4a2),
            text_muted: rgb(0x7a9a73),
            // Accent
            accent: rgb(0x6abf69),
            accent_hover: rgb(0x7dd07c),
            accent_muted: rgba(0x3d5a3a33),
            // Semantic
            success: rgb(0x6abf69),
            warning: rgb(0xe0c062),
            error: rgb(0xd96c6c),
            info: rgb(0x6cb2d9),
            // Border
            border: rgb(0x3a4a35),
            border_hover: rgb(0x556b50),
            // Badge colors (dark variant)
            badge_primary_bg: rgb(0x1a4a7a),
            badge_primary_text: rgb(0x7cc4ff),
            badge_success_bg: rgb(0x1a3a1a),
            badge_success_text: rgb(0x7ccc7c),
            badge_warning_bg: rgb(0x3a3a1a),
            badge_warning_text: rgb(0xcccc7c),
            badge_error_bg: rgb(0x3a1a1a),
            badge_error_text: rgb(0xcc7c7c),
            badge_info_bg: rgb(0x1a3a3a),
            badge_info_text: rgb(0x7ccccc),
        }
    }

    /// Create a black & white theme (monochrome high contrast)
    pub fn black_and_white() -> Self {
        Self {
            variant: ThemeVariant::BlackAndWhite,
            // Backgrounds
            background: rgb(0x000000),
            surface: rgb(0x141414),
            surface_hover: rgb(0x222222),
            muted: rgb(0x0a0a0a),
            transparent: rgba(0x00000000),
            overlay_bg: rgba(0x00000088),
            // Text
            text_primary: rgb(0xffffff),
            text_secondary: rgb(0x888888),
            text_muted: rgb(0x555555),
            // Accent (black background with white border for buttons)
            accent: rgb(0x000000),
            accent_hover: rgb(0x222222),
            accent_muted: rgba(0x33333333),
            // Semantic (grayscale for B&W theme)
            success: rgb(0xaaaaaa),
            warning: rgb(0x888888),
            error: rgb(0x666666),
            info: rgb(0x999999),
            // Border (white for high contrast)
            border: rgb(0xffffff),
            border_hover: rgb(0xcccccc),
            // Badge colors (dark variant)
            badge_primary_bg: rgb(0x1a4a7a),
            badge_primary_text: rgb(0x7cc4ff),
            badge_success_bg: rgb(0x1a3a1a),
            badge_success_text: rgb(0x7ccc7c),
            badge_warning_bg: rgb(0x3a3a1a),
            badge_warning_text: rgb(0xcccc7c),
            badge_error_bg: rgb(0x3a1a1a),
            badge_error_text: rgb(0xcc7c7c),
            badge_info_bg: rgb(0x1a3a3a),
            badge_info_text: rgb(0x7ccccc),
        }
    }

    /// Get theme for variant
    pub fn for_variant(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::Dark => Self::dark(),
            ThemeVariant::Light => Self::light(),
            ThemeVariant::Midnight => Self::midnight(),
            ThemeVariant::Forest => Self::forest(),
            ThemeVariant::BlackAndWhite => Self::black_and_white(),
        }
    }

    // =========================================================================
    // Color Token Accessors
    // =========================================================================

    /// Get a ColorToken for the accent color with auto-generated variants
    ///
    /// Returns a token with base, hover, active, muted, and subtle variants.
    pub fn accent_token(&self) -> ColorToken {
        ColorToken::from_base(self.accent)
    }

    /// Get a ColorToken for the success color
    pub fn success_token(&self) -> ColorToken {
        ColorToken::from_base(self.success)
    }

    /// Get a ColorToken for the warning color
    pub fn warning_token(&self) -> ColorToken {
        ColorToken::from_base(self.warning)
    }

    /// Get a ColorToken for the error color
    pub fn error_token(&self) -> ColorToken {
        ColorToken::from_base(self.error)
    }

    /// Get a ColorToken for the info color
    pub fn info_token(&self) -> ColorToken {
        ColorToken::from_base(self.info)
    }

    /// Get a ColorToken for the surface color
    pub fn surface_token(&self) -> ColorToken {
        ColorToken::from_base(self.surface)
    }

    /// Get a ColorToken for the primary text color
    pub fn text_primary_token(&self) -> ColorToken {
        ColorToken::from_base(self.text_primary)
    }

    /// Get a ColorToken for the border color
    pub fn border_token(&self) -> ColorToken {
        ColorToken::from_base(self.border)
    }

    /// Convert the theme to a full ColorPalette
    ///
    /// This is useful when you need structured access to all color tokens.
    pub fn to_palette(&self) -> ColorPalette {
        ColorPalette {
            semantic: SemanticColors {
                primary: self.accent_token(),
                secondary: ColorToken::from_base(self.text_secondary),
                success: self.success_token(),
                warning: self.warning_token(),
                error: self.error_token(),
                info: self.info_token(),
            },
            backgrounds: BackgroundColors {
                page: ColorToken::from_base(self.background),
                surface: self.surface_token(),
                overlay: ColorToken::from_base(self.overlay_bg),
            },
            text: TextColors {
                primary: self.text_primary_token(),
                secondary: ColorToken::from_base(self.text_secondary),
                muted: ColorToken::from_base(self.text_muted),
                inverted: ColorToken::from_base(if self.variant == ThemeVariant::Light {
                    rgb(0xffffff)
                } else {
                    rgb(0x1a1a1a)
                }),
            },
            borders: BorderColors {
                default: self.border_token(),
                focus: self.accent_token(),
                error: self.error_token(),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Global state for theme management
pub struct ThemeState {
    pub theme: Theme,
}

impl Global for ThemeState {}

impl ThemeState {
    /// Create new theme state with default (dark) theme
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
        }
    }

    /// Create theme state with specific variant
    pub fn with_variant(variant: ThemeVariant) -> Self {
        Self {
            theme: Theme::for_variant(variant),
        }
    }

    /// Set theme variant
    pub fn set_variant(&mut self, variant: ThemeVariant) {
        self.theme = Theme::for_variant(variant);
    }

    /// Toggle between light and dark themes
    pub fn toggle(&mut self) {
        self.set_variant(self.theme.variant.toggle());
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for easy theme access
pub trait ThemeExt {
    /// Get the current theme
    fn theme(&self) -> Theme;
}

impl ThemeExt for App {
    fn theme(&self) -> Theme {
        self.try_global::<ThemeState>()
            .map(|s| s.theme.clone())
            .unwrap_or_else(Theme::dark)
    }
}

// Shadow helpers for hover effects

/// Create a glow shadow effect for hover states.
/// This is a shared helper to avoid duplicating shadow construction
/// across button, accordion, menu, tabs, and other components.
pub fn glow_shadow(color: Rgba) -> Vec<BoxShadow> {
    let glow_inner = Hsla::from(color).alpha(0.6);
    let glow_outer = Hsla::from(color).alpha(0.2);
    vec![
        BoxShadow {
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(4.0),
            spread_radius: px(0.0),
            color: glow_inner,
        },
        BoxShadow {
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(25.0),
            spread_radius: px(2.0),
            color: glow_outer,
        },
    ]
}
