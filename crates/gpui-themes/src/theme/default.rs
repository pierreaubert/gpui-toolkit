use super::misc::COMMUNITY_THEME_SCHEMA_VERSION;

pub(super) fn default_community_theme_schema_version() -> u32 {
    COMMUNITY_THEME_SCHEMA_VERSION
}

pub(super) fn default_design_language() -> String {
    "neutral".to_string()
}
