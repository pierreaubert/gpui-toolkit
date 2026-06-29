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

    /// Get flag emoji
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
