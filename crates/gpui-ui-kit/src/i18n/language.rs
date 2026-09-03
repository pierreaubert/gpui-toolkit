/// Available languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// English (default)
    #[default]
    English,
    /// French
    French,
    /// German
    German,
    /// Spanish
    Spanish,
    /// Japanese
    Japanese,
}

impl Language {
    /// Get all available languages
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::French,
            Language::German,
            Language::Spanish,
            Language::Japanese,
        ]
    }

    /// Get display name in the language itself
    pub fn native_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::French => "Francais",
            Language::German => "Deutsch",
            Language::Spanish => "Espanol",
            Language::Japanese => "Nihongo",
        }
    }

    /// Get language code (ISO 639-1)
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::French => "fr",
            Language::German => "de",
            Language::Spanish => "es",
            Language::Japanese => "ja",
        }
    }
}

impl Language {
    /// Logical layout direction for this language (bidi/RTL mirroring).
    ///
    /// All currently supported languages are LTR; this returns
    /// [`LayoutDirection::Rtl`] once RTL locales are added.
    pub const fn layout_direction(self) -> LayoutDirection {
        LayoutDirection::of(self)
    }

    pub fn flag(&self) -> &'static str {
        match self {
            Language::English => "GB",
            Language::French => "FR",
            Language::German => "DE",
            Language::Spanish => "ES",
            Language::Japanese => "JP",
        }
    }
}

/// Logical layout direction for bidi (RTL) mirroring.
///
/// SOTA kits mirror spacing, pane order, and corner radii under RTL locales
/// via logical properties. Components take this explicitly so headless/test
/// contexts stay deterministic; resolve it from [`Language`] with
/// [`Language::layout_direction`] or [`LayoutDirection::of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LayoutDirection {
    /// Left-to-right (default).
    #[default]
    Ltr,
    /// Right-to-left: horizontal layouts mirror.
    Rtl,
}

impl LayoutDirection {
    /// Resolve the layout direction for a language.
    ///
    /// All currently supported languages are LTR; this returns [`Self::Rtl`]
    /// once RTL locales (e.g. Arabic, Hebrew) are added, keeping the
    /// mirroring plumbing in place ahead of the translations.
    pub const fn of(language: Language) -> Self {
        let _ = language;
        Self::Ltr
    }

    /// Whether horizontal layouts using this direction mirror.
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::Rtl)
    }
}
