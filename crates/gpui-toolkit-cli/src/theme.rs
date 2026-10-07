//! Theme preset export, staleness checks, and token targets.
//!
//! [`theme_build`] exports one [`BuiltInThemePreset`] to Style Dictionary
//! JSON; [`theme_check`] recomputes the bytes and reports missing or
//! outdated outputs for CI; [`theme_targets`] lists the preset's whole
//! token surface derived from the same export, so the table cannot drift
//! from what builds produce.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use crate::init::is_plain_file_name;
use gpui_themes::{BuiltInThemePreset, EditorTheme};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Discriminator for theme build receipts.
pub const THEME_BUILD_TYPE: &str = "theme.build";

/// Discriminator for theme staleness receipts.
pub const THEME_CHECK_TYPE: &str = "theme.build.check";

/// Discriminator for theme target listings.
pub const THEME_TARGETS_TYPE: &str = "theme.targets";

/// Discriminator for theme preset listings.
pub const THEME_LIST_TYPE: &str = "theme.list";

/// One preset row of theme list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeEntry {
    /// Preset id, such as `dark`.
    pub id: String,
    /// Display name, such as `Dark`.
    pub name: String,
}

/// Theme preset list payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeList {
    /// Number of presets.
    pub count: usize,
    /// Presets in registry order.
    pub themes: Vec<ThemeEntry>,
}

/// Theme build receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeBuild {
    /// Preset id that was built.
    pub preset: String,
    /// Exported token count.
    pub token_count: usize,
    /// Written bytes.
    pub bytes: usize,
    /// Written kibibytes, for human summaries.
    pub size_kb: f64,
    /// Written file path.
    pub path: PathBuf,
    /// Export defects to fix; empty when clean.
    pub warnings: Vec<String>,
}

/// One stale output found by [`theme_check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleOutput {
    /// Checked path.
    pub path: PathBuf,
    /// `missing` or `outdated`.
    pub reason: String,
}

/// Theme staleness receipt; writes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeCheck {
    /// Preset id that was checked.
    pub preset: String,
    /// Whether all outputs are current.
    pub up_to_date: bool,
    /// Stale outputs, empty when current.
    pub stale: Vec<StaleOutput>,
    /// Every path that was checked.
    pub checked: Vec<PathBuf>,
}

/// One themeable token target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeTarget {
    /// Token key, such as `color.accent`.
    pub key: String,
    /// Dotted path segments.
    pub path: Vec<String>,
    /// Token type, such as `color`.
    pub token_type: String,
}

/// Token surface payload for one preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeTargets {
    /// Preset id.
    pub preset: String,
    /// Targets sorted by key.
    pub targets: Vec<ThemeTarget>,
}

/// Lists all built-in presets in registry order.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::theme_list;
///
/// let list = theme_list();
/// assert!(list.themes.iter().any(|entry| entry.id == "dark"));
/// ```
pub fn theme_list() -> ThemeList {
    ThemeList {
        count: BuiltInThemePreset::all().len(),
        themes: BuiltInThemePreset::all()
            .iter()
            .map(|preset| ThemeEntry {
                id: preset.id().to_owned(),
                name: preset.name().to_owned(),
            })
            .collect(),
    }
}

/// Resolves a preset id or fails with `ERR_UNKNOWN_THEME`.
fn lookup_preset(id: &str) -> Result<BuiltInThemePreset, ToolkitError> {
    BuiltInThemePreset::from_id(id).ok_or_else(|| {
        ToolkitError::new(
            ErrorCode::UnknownTheme,
            format!("no theme preset named '{id}'"),
        )
    })
}

/// Renders one preset to deterministic Style Dictionary JSON.
fn render_preset(preset: BuiltInThemePreset) -> Result<(String, usize), ToolkitError> {
    let theme = EditorTheme::preset(preset);
    let tokens = theme.style_dictionary_tokens();
    let json = theme
        .to_style_dictionary_json()
        .map_err(|error| ToolkitError::new(ErrorCode::Unknown, error.to_string()))?;
    Ok((json, tokens.len()))
}

/// Default output file name for a preset export.
fn default_theme_file(preset: BuiltInThemePreset) -> String {
    format!("{}.tokens.json", preset.id())
}

/// Exports one preset to Style Dictionary JSON.
///
/// Writes `<id>.tokens.json` (or `file`) into `dir`, creating nothing
/// else. Existing files are overwritten: builds are reproducible, so
/// reruns converge instead of conflicting.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTheme`] for unknown presets,
/// [`ErrorCode::InvalidArgument`] for a missing directory or a
/// non-plain file name, and [`ErrorCode::WriteFailed`] for I/O
/// failures.
pub fn theme_build(
    preset_id: &str,
    dir: &Path,
    file: Option<&str>,
) -> Result<ThemeBuild, ToolkitError> {
    let preset = lookup_preset(preset_id)?;
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let file_name = file.map_or_else(|| default_theme_file(preset), str::to_owned);
    if !is_plain_file_name(&file_name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("file name '{file_name}' must be a plain file name"),
        ));
    }
    let (json, token_count) = render_preset(preset)?;
    let path = dir.join(file_name);
    std::fs::write(&path, &json).map_err(|error| {
        ToolkitError::new(
            ErrorCode::WriteFailed,
            format!("cannot write '{}': {error}", path.display()),
        )
    })?;
    Ok(ThemeBuild {
        preset: preset.id().to_owned(),
        token_count,
        bytes: json.len(),
        size_kb: json.len() as f64 / 1024.0,
        path,
        warnings: Vec::new(),
    })
}

/// Checks whether a preset export is current without writing.
///
/// Recomputes the expected bytes and compares them with the file:
/// missing files report `missing`, differing bytes report `outdated`.
///
/// # Errors
///
/// Returns the same errors as [`theme_build`], except nothing is
/// written and unreadable files report `missing`.
pub fn theme_check(
    preset_id: &str,
    dir: &Path,
    file: Option<&str>,
) -> Result<ThemeCheck, ToolkitError> {
    let preset = lookup_preset(preset_id)?;
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let file_name = file.map_or_else(|| default_theme_file(preset), str::to_owned);
    if !is_plain_file_name(&file_name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("file name '{file_name}' must be a plain file name"),
        ));
    }
    let (expected, _) = render_preset(preset)?;
    let path = dir.join(file_name);
    let stale = match std::fs::read_to_string(&path) {
        Ok(actual) if actual == expected => Vec::new(),
        Ok(_) => vec![StaleOutput {
            path: path.clone(),
            reason: String::from("outdated"),
        }],
        Err(_) => vec![StaleOutput {
            path: path.clone(),
            reason: String::from("missing"),
        }],
    };
    Ok(ThemeCheck {
        preset: preset.id().to_owned(),
        up_to_date: stale.is_empty(),
        stale,
        checked: vec![path],
    })
}

/// Lists the token surface of one preset.
///
/// Targets derive from the same export [`theme_build`] writes, so the
/// table always matches build output.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTheme`] for unknown presets.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::theme_targets;
///
/// let targets = theme_targets("dark").unwrap();
/// assert!(targets.targets.iter().any(|target| target.key == "color.accent"));
/// ```
pub fn theme_targets(preset_id: &str) -> Result<ThemeTargets, ToolkitError> {
    let preset = lookup_preset(preset_id)?;
    let theme = EditorTheme::preset(preset);
    let targets = theme
        .style_dictionary_tokens()
        .into_iter()
        .map(|token| ThemeTarget {
            key: token.name,
            path: token.path,
            token_type: token.token_type,
        })
        .collect();
    Ok(ThemeTargets {
        preset: preset.id().to_owned(),
        targets,
    })
}

/// Renders a theme list as human-readable text.
pub fn render_theme_list_text(list: &ThemeList, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!("{} presets:\n\n", list.count));
    }
    for entry in &list.themes {
        if dense {
            text.push_str(&format!("{}\n", entry.id));
        } else {
            text.push_str(&format!("{} — {}\n", entry.id, entry.name));
        }
    }
    text
}

/// Renders a build receipt as human-readable text.
pub fn render_theme_build_text(build: &ThemeBuild) -> String {
    format!(
        "Exported {} tokens of '{}' to {} ({:.1} KB)\n",
        build.token_count,
        build.preset,
        build.path.display(),
        build.size_kb
    )
}

/// Renders a staleness receipt as human-readable text.
pub fn render_theme_check_text(check: &ThemeCheck) -> String {
    if check.up_to_date {
        return format!("'{}' is up to date\n", check.preset);
    }
    let mut text = format!("'{}' is stale:\n", check.preset);
    for stale in &check.stale {
        text.push_str(&format!("  {} ({})\n", stale.path.display(), stale.reason));
    }
    text
}

/// Renders token targets as human-readable text.
pub fn render_theme_targets_text(targets: &ThemeTargets, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!(
            "{} targets for '{}':\n\n",
            targets.targets.len(),
            targets.preset
        ));
    }
    for target in &targets.targets {
        if dense {
            text.push_str(&format!("{}\n", target.key));
        } else {
            text.push_str(&format!("{} [{}]\n", target.key, target.token_type));
        }
    }
    text
}
