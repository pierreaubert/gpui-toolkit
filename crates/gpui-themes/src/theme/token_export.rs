use super::editor_theme::EditorTheme;
use super::misc::Color;
use serde::{Deserialize, Serialize};

/// A single design token in Style Dictionary shape.
///
/// Mirrors `gpui_design::DesignToken` (`name`, dotted `path`, hex `value`,
/// `token_type`) so theme exports round-trip with `gpui-design` tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDesignToken {
    pub name: String,
    pub path: Vec<String>,
    pub value: String,
    pub token_type: String,
}

/// MUI-style token alias (`alias` -> theme field key).
///
/// Aliases give tiered access (`palette.primary.main`) over the flat
/// `EditorTheme` fields.
pub fn token_aliases() -> &'static [(&'static str, &'static str)] {
    use std::sync::OnceLock;
    static ALIASES: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        vec![
            ("palette.primary.main", "accent"),
            ("palette.primary.hover", "accent-hover"),
            ("palette.primary.muted", "accent-muted"),
            ("palette.background.default", "background"),
            ("palette.background.paper", "surface"),
            ("palette.background.sunken", "background-secondary"),
            ("palette.text.primary", "text-primary"),
            ("palette.text.secondary", "text-secondary"),
            ("palette.text.disabled", "text-disabled"),
            ("palette.divider", "border"),
            ("palette.action.focus", "border-focused"),
            ("palette.success.main", "success"),
            ("palette.warning.main", "warning"),
            ("palette.error.main", "error"),
            ("palette.info.main", "info"),
        ]
    })
}

impl EditorTheme {
    /// Look up a core color field by its token key (e.g. `"accent"`).
    /// Returns `None` for unknown keys.
    pub fn named_color(&self, key: &str) -> Option<Color> {
        Some(match key {
            "background" => self.background,
            "background-secondary" => self.background_secondary,
            "background-tertiary" => self.background_tertiary,
            "surface" => self.surface,
            "surface-hover" => self.surface_hover,
            "surface-selected" => self.surface_selected,
            "text-primary" => self.text_primary,
            "text-secondary" => self.text_secondary,
            "text-muted" => self.text_muted,
            "text-disabled" => self.text_disabled,
            "border" => self.border,
            "border-focused" => self.border_focused,
            "accent" => self.accent,
            "accent-hover" => self.accent_hover,
            "accent-muted" => self.accent_muted,
            "text-on-accent" => self.text_on_accent,
            "success" => self.success,
            "warning" => self.warning,
            "error" => self.error,
            "info" => self.info,
            _ => return None,
        })
    }

    /// Resolve an MUI-style alias such as `"palette.primary.main"`.
    /// Returns `None` for unknown aliases.
    pub fn resolve_token_alias(&self, alias: &str) -> Option<Color> {
        let key = token_aliases()
            .iter()
            .find(|(name, _)| *name == alias)
            .map(|(_, key)| *key)?;
        self.named_color(key)
    }

    fn core_token_pairs(&self) -> Vec<(&'static str, Color)> {
        vec![
            ("background", self.background),
            ("background-secondary", self.background_secondary),
            ("background-tertiary", self.background_tertiary),
            ("surface", self.surface),
            ("surface-hover", self.surface_hover),
            ("surface-selected", self.surface_selected),
            ("text-primary", self.text_primary),
            ("text-secondary", self.text_secondary),
            ("text-muted", self.text_muted),
            ("text-disabled", self.text_disabled),
            ("border", self.border),
            ("border-focused", self.border_focused),
            ("accent", self.accent),
            ("accent-hover", self.accent_hover),
            ("accent-muted", self.accent_muted),
            ("text-on-accent", self.text_on_accent),
            ("success", self.success),
            ("warning", self.warning),
            ("error", self.error),
            ("info", self.info),
        ]
    }

    /// Export core tokens in Style Dictionary shape (sorted by name).
    pub fn style_dictionary_tokens(&self) -> Vec<ThemeDesignToken> {
        let mut tokens: Vec<ThemeDesignToken> = self
            .core_token_pairs()
            .into_iter()
            .map(|(key, color)| {
                let name = format!("color.{key}");
                ThemeDesignToken {
                    path: name.split('.').map(str::to_string).collect(),
                    name,
                    value: color.to_hex_string(),
                    token_type: "color".to_string(),
                }
            })
            .collect();
        tokens.sort_by(|a, b| a.name.cmp(&b.name));
        tokens
    }

    /// Serialize core tokens as Style Dictionary JSON.
    pub fn to_style_dictionary_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&serde_json::json!({
            "color": self.style_dictionary_tokens().iter().map(|token| {
                serde_json::json!({
                    "name": token.name,
                    "path": token.path,
                    "value": token.value,
                    "type": token.token_type,
                })
            }).collect::<Vec<_>>(),
        }))
    }

    /// Export core colors as CSS custom properties under `:root`.
    pub fn to_css_variables(&self) -> String {
        let mut out = String::from(":root {\n");
        for (key, color) in self.core_token_pairs() {
            out.push_str(&format!("  --color-{key}: {};\n", color.to_hex_string()));
        }
        for (alias, key) in token_aliases() {
            if let Some(color) = self.named_color(key) {
                let var = alias.replace('.', "-");
                out.push_str(&format!("  --{var}: {};\n", color.to_hex_string()));
            }
        }
        out.push_str("}\n");
        out
    }
}
