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
            ThemeVariant::CarbonGray100 => ThemeVariant::Dark,
        }
    }
}
