use super::accessibility_palette::AccessibilityPalette;
use super::default::default_community_theme_schema_version;
use super::editor_theme::EditorTheme;
use super::misc::COMMUNITY_THEME_SCHEMA_VERSION;
use super::misc::slugify_theme_name;
use super::theme_mode_preference::ThemeModePreference;
use super::theme_transition::ThemeTransition;
use super::types::AccentSource;
use serde::{Deserialize, Serialize};

/// Metadata wrapper for shareable JSON theme files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommunityThemeManifest {
    #[serde(default = "default_community_theme_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub accessibility: AccessibilityPalette,
    #[serde(default)]
    pub preferred_mode: ThemeModePreference,
    #[serde(default)]
    pub accent_source: AccentSource,
    #[serde(default)]
    pub transition: ThemeTransition,
}

impl CommunityThemeManifest {
    pub fn for_theme(theme: &EditorTheme) -> Self {
        Self {
            schema_version: COMMUNITY_THEME_SCHEMA_VERSION,
            id: slugify_theme_name(&theme.name),
            display_name: theme.name.clone(),
            author: String::new(),
            license: String::new(),
            tags: Vec::new(),
            accessibility: AccessibilityPalette::Standard,
            preferred_mode: ThemeModePreference::default(),
            accent_source: AccentSource::Theme,
            transition: ThemeTransition::default(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMMUNITY_THEME_SCHEMA_VERSION {
            return Err(format!(
                "unsupported community theme schema version {}",
                self.schema_version
            ));
        }
        if self.id.trim().is_empty() {
            return Err("theme manifest id must not be empty".to_string());
        }
        if self.display_name.trim().is_empty() {
            return Err("theme manifest display_name must not be empty".to_string());
        }
        Ok(())
    }
}
