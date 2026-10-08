//! `toolkit.toml` project configuration, strictly parsed.
//!
//! Projects opt into toolkit behavior with a `toolkit.toml` file:
//! custom layout components under `[layout.components]`, and project
//! metadata under `[project]`. Loading discovers the file by walking up
//! from a starting directory, so nested commands find the project root;
//! `--config` pins one explicitly. Parsing is strict — unknown tables
//! and keys fail with [`ErrorCode::InvalidConfig`] — so typos surface
//! instead of silently doing nothing.
//!
//! ```toml
//! [project]
//! name = "my-app"
//! issues_url = "https://github.com/acme/my-app/issues"
//!
//! [[layout.components]]
//! name = "PrimaryButton"
//! base = "B"
//! modifier = "primary"
//! ```

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use crate::upgrade::{UpgradeRule, builtin_rules};
use gpui_layout_expr::{CustomComponent, CustomComponents, component_kind};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Config file name discovered by walking up from a directory.
pub const CONFIG_FILE_NAME: &str = "toolkit.toml";

/// Root `toolkit.toml` document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolkitConfig {
    /// Project metadata.
    #[serde(default)]
    pub project: ProjectConfig,
    /// Layout expression customization.
    #[serde(default)]
    pub layout: LayoutConfig,
    /// Upgrade detection customization.
    #[serde(default)]
    pub upgrade: UpgradeConfig,
}

/// Project metadata from `[project]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Human-readable project name.
    pub name: Option<String>,
    /// Tracker URL overriding the default gap-report destination.
    pub issues_url: Option<String>,
}

/// Layout customization from `[layout]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutConfig {
    /// Custom component aliases (`[[layout.components]]`).
    #[serde(default)]
    pub components: Vec<LayoutComponentConfig>,
}

/// One custom layout component alias.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutComponentConfig {
    /// Name as used in expressions, such as `PrimaryButton`.
    pub name: String,
    /// Built-in base: one of `V`, `H`, `B`, `I`, `Tx`, `D`.
    pub base: String,
    /// Preset dot modifier (buttons only).
    pub modifier: Option<String>,
}

/// Upgrade detection customization from `[upgrade]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeConfig {
    /// Extra detection rules (`[[upgrade.rules]]`).
    #[serde(default)]
    pub rules: Vec<UpgradeRuleConfig>,
}

/// One custom deprecated-pattern rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeRuleConfig {
    /// Stable rule id, such as `no-legacy-color`.
    pub id: String,
    /// Literal substring matched against code lines.
    pub pattern: String,
    /// Why the pattern is deprecated.
    pub message: String,
    /// Remediation hint.
    pub fix: Option<String>,
}

/// A loaded config plus the path it came from, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    /// Config file path; `None` when no file was found.
    pub path: Option<PathBuf>,
    /// Parsed configuration (defaults when no file was found).
    pub config: ToolkitConfig,
}

impl LoadedConfig {
    /// Builds the custom layout registry from this config.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::InvalidConfig`] for unknown bases and for
    /// every registry violation (bad names, builtin collisions,
    /// duplicates, illegal preset modifiers).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::{LoadedConfig, ToolkitConfig};
    ///
    /// let loaded = LoadedConfig { path: None, config: ToolkitConfig::default() };
    /// assert!(loaded.customs().unwrap().is_empty());
    /// ```
    pub fn customs(&self) -> Result<CustomComponents, ToolkitError> {
        let mut specs = Vec::with_capacity(self.config.layout.components.len());
        for component in &self.config.layout.components {
            let base = component_kind(&component.base).ok_or_else(|| {
                ToolkitError::new(
                    ErrorCode::InvalidConfig,
                    format!(
                        "layout component '{}' has unknown base '{}' (want one of V H B I Tx D)",
                        component.name, component.base
                    ),
                )
            })?;
            specs.push(CustomComponent {
                name: component.name.clone(),
                base,
                modifier: component.modifier.clone(),
            });
        }
        CustomComponents::new(specs).map_err(|error| {
            ToolkitError::new(
                ErrorCode::InvalidConfig,
                format!("invalid layout components: {}", error.message),
            )
        })
    }

    /// Builds the extra upgrade detection rules from this config.
    ///
    /// Rule ids must be unique across built-ins and config, patterns
    /// must be non-empty, and ids use lowercase letters, digits,
    /// dashes, and underscores.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::InvalidConfig`] for blank ids, bad id
    /// characters, duplicate ids, and empty patterns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::{LoadedConfig, ToolkitConfig};
    ///
    /// let loaded = LoadedConfig { path: None, config: ToolkitConfig::default() };
    /// assert!(loaded.upgrade_rules().unwrap().is_empty());
    /// ```
    pub fn upgrade_rules(&self) -> Result<Vec<UpgradeRule>, ToolkitError> {
        let builtins = builtin_rules();
        let mut rules = Vec::with_capacity(self.config.upgrade.rules.len());
        for rule in &self.config.upgrade.rules {
            if rule.id.is_empty()
                || !rule.id.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_'
                })
            {
                return Err(ToolkitError::new(
                    ErrorCode::InvalidConfig,
                    format!(
                        "upgrade rule '{}' needs a kebab-case id (lowercase, digits, '-', '_')",
                        rule.id
                    ),
                ));
            }
            if rule.pattern.is_empty() {
                return Err(ToolkitError::new(
                    ErrorCode::InvalidConfig,
                    format!("upgrade rule '{}' needs a non-empty pattern", rule.id),
                ));
            }
            if builtins.iter().any(|known| known.id == rule.id)
                || rules.iter().any(|known: &UpgradeRule| known.id == rule.id)
            {
                return Err(ToolkitError::new(
                    ErrorCode::InvalidConfig,
                    format!("upgrade rule '{}' is defined twice", rule.id),
                ));
            }
            rules.push(UpgradeRule {
                id: rule.id.clone(),
                pattern: rule.pattern.clone(),
                message: rule.message.clone(),
                fix: rule.fix.clone(),
            });
        }
        Ok(rules)
    }
}

/// Parses `toolkit.toml` text with strict unknown-field rejection.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidConfig`] when the text is not valid
/// TOML or carries unknown tables or keys.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::parse_config;
///
/// let config = parse_config("[project]\nname = \"demo\"\n").unwrap();
/// assert_eq!(config.project.name.as_deref(), Some("demo"));
/// parse_config("[surprise]\n").unwrap_err();
/// ```
pub fn parse_config(text: &str) -> Result<ToolkitConfig, ToolkitError> {
    toml::from_str(text).map_err(|error| {
        ToolkitError::new(
            ErrorCode::InvalidConfig,
            format!("cannot parse {CONFIG_FILE_NAME}: {error}"),
        )
    })
}

/// Finds `toolkit.toml` by walking up from `start`.
///
/// Returns the nearest match, or `None` when no ancestor carries the
/// file. A `start` that is itself the file also matches.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::discover_config;
///
/// assert!(discover_config(std::path::Path::new("/definitely/not/a/toolkit/dir")).is_none());
/// ```
pub fn discover_config(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = dir.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Reads and parses one config file.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidConfig`] when the file cannot be read
/// or fails strict parsing.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::read_config_file;
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("toolkit.toml");
/// std::fs::write(&path, "[project]\nname = \"demo\"\n").unwrap();
/// let loaded = read_config_file(&path).unwrap();
/// assert_eq!(loaded.config.project.name.as_deref(), Some("demo"));
/// ```
pub fn read_config_file(path: &Path) -> Result<LoadedConfig, ToolkitError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        ToolkitError::new(
            ErrorCode::InvalidConfig,
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    let config = parse_config(&text)?;
    Ok(LoadedConfig {
        path: Some(path.to_path_buf()),
        config,
    })
}

/// Loads the active config: explicit `--config` or discovered file.
///
/// Without an explicit path, discovery walks up from the working
/// directory; when nothing is found the result carries defaults and no
/// path.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidConfig`] when the explicit file cannot
/// be read or any loaded file fails strict parsing, and
/// [`ErrorCode::Unknown`] when the working directory is unavailable.
pub fn load_config(explicit: Option<&Path>) -> Result<LoadedConfig, ToolkitError> {
    if let Some(path) = explicit {
        return read_config_file(path);
    }
    let cwd = std::env::current_dir().map_err(|error| {
        ToolkitError::new(
            ErrorCode::Unknown,
            format!("cannot determine working directory: {error}"),
        )
    })?;
    match discover_config(&cwd) {
        Some(path) => read_config_file(&path),
        None => Ok(LoadedConfig {
            path: None,
            config: ToolkitConfig::default(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, LoadedConfig, parse_config, read_config_file};

    #[test]
    fn parses_full_config() {
        let text = "[project]\nname = \"demo\"\nissues_url = \"https://x.test/i\"\n\n\
            [[layout.components]]\nname = \"PrimaryButton\"\nbase = \"B\"\nmodifier = \"primary\"\n";
        let config = parse_config(text).unwrap();
        assert_eq!(config.project.name.as_deref(), Some("demo"));
        assert_eq!(
            config.project.issues_url.as_deref(),
            Some("https://x.test/i")
        );
        assert_eq!(config.layout.components.len(), 1);
        let loaded = LoadedConfig { path: None, config };
        assert_eq!(loaded.customs().unwrap().names(), vec!["PrimaryButton"]);
    }

    #[test]
    fn rejects_unknown_fields_and_bases() {
        parse_config("[project]\ntypo = 1\n").unwrap_err();
        parse_config("[layout]\ncomponents = 1\n").unwrap_err();
        let config =
            parse_config("[[layout.components]]\nname = \"Alias\"\nbase = \"Table\"\n").unwrap();
        let loaded = LoadedConfig { path: None, config };
        let error = loaded.customs().unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidConfig);
        assert!(error.message.contains("unknown base"));
    }

    #[test]
    fn rejects_bad_registry_specs() {
        let config = parse_config("[[layout.components]]\nname = \"B\"\nbase = \"B\"\n").unwrap();
        let loaded = LoadedConfig { path: None, config };
        loaded.customs().unwrap_err();
    }

    #[test]
    fn missing_file_is_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let error = read_config_file(&dir.path().join("toolkit.toml")).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn rejects_bad_upgrade_rules() {
        let config = parse_config(
            "[[upgrade.rules]]\nid = \"no-build-with-theme\"\npattern = \"x\"\nmessage = \"y\"\n",
        )
        .unwrap();
        LoadedConfig { path: None, config }
            .upgrade_rules()
            .unwrap_err();
        let config =
            parse_config("[[upgrade.rules]]\nid = \"Bad Id\"\npattern = \"x\"\nmessage = \"y\"\n")
                .unwrap();
        LoadedConfig { path: None, config }
            .upgrade_rules()
            .unwrap_err();
        let config =
            parse_config("[[upgrade.rules]]\nid = \"ok\"\npattern = \"\"\nmessage = \"y\"\n")
                .unwrap();
        LoadedConfig { path: None, config }
            .upgrade_rules()
            .unwrap_err();
    }
}
