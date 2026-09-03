//! Opt-in persistence for MiniApp session state (window size, theme, language).
//!
//! The state is stored as a small `key=value` text file so the shell needs no
//! new dependencies (pulling `serde`/`dirs` in here would leak into every demo
//! build that embeds this crate):
//!
//! ```text
//! width=900
//! height=700
//! theme=Dark
//! language=en
//! ```
//!
//! Unknown keys and malformed values are ignored line by line, so a partially
//! corrupt file still applies its valid lines. A missing or unreadable file
//! loads as [`None`] and callers keep their existing (builder) values.

use gpui_ui_kit::i18n::Language;
use gpui_ui_kit::theme::ThemeVariant;
use std::fmt::Write as _;
use std::path::Path;

/// Session values restored from a state file.
///
/// Each field is [`None`] when the file did not provide a usable value, in
/// which case the caller keeps its existing (builder or default) value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MiniAppState {
    /// Window width in pixels, when present and positive.
    pub width: Option<f32>,
    /// Window height in pixels, when present and positive.
    pub height: Option<f32>,
    /// Theme variant, when present and recognized.
    pub theme: Option<ThemeVariant>,
    /// Language, when present and recognized.
    pub language: Option<Language>,
}

impl MiniAppState {
    /// Concrete snapshot of live values, for saving.
    #[must_use]
    pub fn snapshot(width: f32, height: f32, theme: ThemeVariant, language: Language) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
            theme: Some(theme),
            language: Some(language),
        }
    }

    /// True when no field carries a value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width.is_none()
            && self.height.is_none()
            && self.theme.is_none()
            && self.language.is_none()
    }
}

/// Parse a theme previously written via [`ThemeVariant::name`].
///
/// Matching is case-insensitive and surrounding whitespace is ignored, so
/// values like `"dark"`, `" Dark "`, and `"Black & White"` all resolve.
#[must_use]
pub fn theme_from_name(name: &str) -> Option<ThemeVariant> {
    let name = name.trim();
    ThemeVariant::all()
        .iter()
        .copied()
        .find(|variant| variant.name().eq_ignore_ascii_case(name))
}

/// Parse a language previously written via [`Language::code`].
///
/// Matching is case-insensitive and surrounding whitespace is ignored, so
/// values like `"EN"`, `" fr "`, and `"ja"` all resolve.
#[must_use]
pub fn language_from_code(code: &str) -> Option<Language> {
    let code = code.trim().to_ascii_lowercase();
    Language::all()
        .iter()
        .copied()
        .find(|language| language.code() == code)
}

fn parse_positive_size(value: &str) -> Option<f32> {
    match value.trim().parse::<f32>() {
        Ok(size) if size.is_finite() && size > 0.0 => Some(size),
        _ => None,
    }
}

/// Load session state from `path`.
///
/// Returns [`None`] when the file cannot be read. A readable file with no
/// usable lines yields an empty (all-[`None`]) state rather than an error, so
/// corrupt files degrade to builder values instead of failing startup.
///
/// Unknown keys, lines without `=`, and malformed values are skipped; later
/// valid lines still apply.
#[must_use]
pub fn load_miniapp_state(path: &Path) -> Option<MiniAppState> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut state = MiniAppState::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "width" => state.width = parse_positive_size(value).or(state.width),
            "height" => state.height = parse_positive_size(value).or(state.height),
            "theme" => state.theme = theme_from_name(value).or(state.theme),
            "language" => state.language = language_from_code(value).or(state.language),
            _ => {}
        }
    }
    Some(state)
}

/// Save session state to `path`, creating parent directories as needed.
///
/// Only fields holding a value are written; an empty state writes just the
/// header comment.
///
/// # Errors
///
/// Returns the underlying I/O error when directories cannot be created or the
/// file cannot be written.
pub fn save_miniapp_state(path: &Path, state: &MiniAppState) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::from("# MiniApp session state (generated; safe to delete)\n");
    if let Some(width) = state.width {
        let _ = writeln!(text, "width={width}");
    }
    if let Some(height) = state.height {
        let _ = writeln!(text, "height={height}");
    }
    if let Some(theme) = state.theme {
        let _ = writeln!(text, "theme={}", theme.name());
    }
    if let Some(language) = state.language {
        let _ = writeln!(text, "language={}", language.code());
    }
    std::fs::write(path, text)
}
