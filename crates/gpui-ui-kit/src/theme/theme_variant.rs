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
    /// Onyx theme (near-black with warm amber/gold accent)
    Onyx,
    /// Carbon White theme.
    CarbonWhite,
    /// Carbon Gray 10 light theme.
    CarbonGray10,
    /// Carbon Gray 90 dark theme.
    CarbonGray90,
    /// Carbon Gray 100 dark theme.
    CarbonGray100,
    /// Native macOS dark theme (`AppKit` semantic colors).
    MacosDark,
    /// Native macOS light theme (`AppKit` semantic colors).
    MacosLight,
    /// Native Windows 11 dark theme (Fluent theme resources).
    FluentDark,
    /// Native Windows 11 light theme (Fluent theme resources).
    FluentLight,
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
            ThemeVariant::Onyx,
            ThemeVariant::CarbonWhite,
            ThemeVariant::CarbonGray10,
            ThemeVariant::CarbonGray90,
            ThemeVariant::CarbonGray100,
            ThemeVariant::MacosDark,
            ThemeVariant::MacosLight,
            ThemeVariant::FluentDark,
            ThemeVariant::FluentLight,
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
            ThemeVariant::Onyx => "Onyx",
            ThemeVariant::CarbonWhite => "Carbon White",
            ThemeVariant::CarbonGray10 => "Carbon Gray 10",
            ThemeVariant::CarbonGray90 => "Carbon Gray 90",
            ThemeVariant::CarbonGray100 => "Carbon Gray 100",
            ThemeVariant::MacosDark => "macOS Dark",
            ThemeVariant::MacosLight => "macOS Light",
            ThemeVariant::FluentDark => "Fluent Dark",
            ThemeVariant::FluentLight => "Fluent Light",
        }
    }

    /// Toggle to next variant
    pub fn toggle(&self) -> Self {
        match self {
            ThemeVariant::Dark => ThemeVariant::Light,
            ThemeVariant::Light => ThemeVariant::Midnight,
            ThemeVariant::Midnight => ThemeVariant::Forest,
            ThemeVariant::Forest => ThemeVariant::BlackAndWhite,
            ThemeVariant::BlackAndWhite => ThemeVariant::Onyx,
            ThemeVariant::Onyx => ThemeVariant::CarbonWhite,
            ThemeVariant::CarbonWhite => ThemeVariant::CarbonGray10,
            ThemeVariant::CarbonGray10 => ThemeVariant::CarbonGray90,
            ThemeVariant::CarbonGray90 => ThemeVariant::CarbonGray100,
            ThemeVariant::CarbonGray100 => ThemeVariant::MacosDark,
            ThemeVariant::MacosDark => ThemeVariant::MacosLight,
            ThemeVariant::MacosLight => ThemeVariant::FluentDark,
            ThemeVariant::FluentDark => ThemeVariant::FluentLight,
            ThemeVariant::FluentLight => ThemeVariant::Dark,
        }
    }

    /// Whether this variant is a dark appearance.
    pub fn is_dark(&self) -> bool {
        match self {
            ThemeVariant::Dark => true,
            ThemeVariant::Light => false,
            ThemeVariant::Midnight => true,
            ThemeVariant::Forest => true,
            ThemeVariant::BlackAndWhite => true,
            ThemeVariant::Onyx => true,
            ThemeVariant::CarbonWhite => false,
            ThemeVariant::CarbonGray10 => false,
            ThemeVariant::CarbonGray90 => true,
            ThemeVariant::CarbonGray100 => true,
            ThemeVariant::MacosDark => true,
            ThemeVariant::MacosLight => false,
            ThemeVariant::FluentDark => true,
            ThemeVariant::FluentLight => false,
        }
    }

    /// Platform-native variant for an OS dark-mode flag.
    ///
    /// macOS resolves to the `Macos` pair and Windows to the `Fluent`
    /// pair; every other platform keeps the classic `Dark`/`Light` pair,
    /// so Linux rendering is unchanged.
    pub fn for_platform_appearance(system_dark: bool) -> Self {
        #[cfg(target_os = "macos")]
        {
            if system_dark {
                Self::MacosDark
            } else {
                Self::MacosLight
            }
        }
        #[cfg(target_os = "windows")]
        {
            if system_dark {
                Self::FluentDark
            } else {
                Self::FluentLight
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            if system_dark { Self::Dark } else { Self::Light }
        }
    }

    /// Platform-native variant for a GPUI window appearance.
    ///
    /// Vibrant appearances map to their base side: vibrant dark counts
    /// as dark, vibrant light as light.
    pub fn for_window_appearance(appearance: gpui::WindowAppearance) -> Self {
        Self::for_platform_appearance(matches!(
            appearance,
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ))
    }

    /// Variant tracking a live OS appearance change.
    ///
    /// Stays within the current family: the `Macos`, `Fluent`, and
    /// classic pairs swap sides, while fixed product themes such as
    /// `Midnight` are returned unchanged.
    pub fn with_system_appearance(self, system_dark: bool) -> Self {
        match self {
            ThemeVariant::MacosDark | ThemeVariant::MacosLight => {
                if system_dark {
                    Self::MacosDark
                } else {
                    Self::MacosLight
                }
            }
            ThemeVariant::FluentDark | ThemeVariant::FluentLight => {
                if system_dark {
                    Self::FluentDark
                } else {
                    Self::FluentLight
                }
            }
            ThemeVariant::Dark | ThemeVariant::Light => {
                if system_dark {
                    Self::Dark
                } else {
                    Self::Light
                }
            }
            ThemeVariant::Midnight => Self::Midnight,
            ThemeVariant::Forest => Self::Forest,
            ThemeVariant::BlackAndWhite => Self::BlackAndWhite,
            ThemeVariant::Onyx => Self::Onyx,
            ThemeVariant::CarbonWhite => Self::CarbonWhite,
            ThemeVariant::CarbonGray10 => Self::CarbonGray10,
            ThemeVariant::CarbonGray90 => Self::CarbonGray90,
            ThemeVariant::CarbonGray100 => Self::CarbonGray100,
        }
    }
}
