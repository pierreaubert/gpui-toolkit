use super::built_in_theme_preset::BuiltInThemePreset;
use super::community_theme_bundle::CommunityThemeBundle;
use super::types::ThemeGalleryEntry;
use serde::{Deserialize, Serialize};

/// Lightweight gallery manifest derived from built-ins and community bundles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeGallery {
    pub entries: Vec<ThemeGalleryEntry>,
}

impl ThemeGallery {
    pub fn from_built_ins() -> Self {
        let entries = BuiltInThemePreset::all()
            .iter()
            .map(|preset| ThemeGalleryEntry {
                id: preset.id().to_string(),
                display_name: preset.name().to_string(),
                tags: vec!["built-in".to_string()],
                accessibility: preset.accessibility(),
                appearance: preset.appearance(),
            })
            .collect();
        Self { entries }
    }

    pub fn with_community_bundle(mut self, bundle: &CommunityThemeBundle) -> Self {
        self.entries.retain(|entry| entry.id != bundle.manifest.id);
        self.entries.push(ThemeGalleryEntry {
            id: bundle.manifest.id.clone(),
            display_name: bundle.manifest.display_name.clone(),
            tags: bundle.manifest.tags.clone(),
            accessibility: bundle.manifest.accessibility,
            appearance: bundle.theme.appearance(),
        });
        self.entries
            .sort_by(|a, b| a.display_name.cmp(&b.display_name));
        self
    }
}
