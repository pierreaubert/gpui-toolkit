use super::language::Language;
use super::types::TranslationKey;

/// Extension trait for easy i18n access
pub trait I18nExt {
    /// Get translation for current language
    fn t(&self, key: TranslationKey) -> &'static str;

    /// Get current language
    fn language(&self) -> Language;
}
