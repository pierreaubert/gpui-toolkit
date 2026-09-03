//! Internationalization (i18n) system for gpui-ui-kit
//!
//! Provides translation support with multiple languages.

mod i18n_ext;
mod i18n_state;
mod language;
mod translations;
mod types;

pub use i18n_ext::I18nExt;
pub use i18n_state::I18nState;
pub use language::{Language, LayoutDirection};
pub use translations::Translations;
pub use types::TranslationKey;
