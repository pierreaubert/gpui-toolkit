# gpui-themes

Theme editor and management for GPUI applications.

Serializable theme system with JSON and Rust export support. Includes a color picker, component showcase, and built-in theme editor for creating and previewing themes.

## Review Gap Coverage

- Accessibility presets: `HighContrast`, `Protanopia`, `Deuteranopia`, and `Tritanopia` are first-class `BuiltInThemePreset` values.
- System accent integration: platform code can pass an OS, wallpaper, or user seed through `AccentPalette::from_seed` and apply it with `EditorTheme::with_accent_palette`.
- Per-app mode overrides: `ThemeModePreference` resolves `follow_system`, forced light/dark, and scheduled appearance modes.
- Community sharing: `CommunityThemeBundle` wraps an `EditorTheme` with schema-versioned manifest metadata for gallery/import/export workflows.
- Transition policy: `ThemeTransition` carries duration, easing, cross-fade, and reduced-motion handling.

## Community JSON Shape

```json
{
  "manifest": {
    "schema_version": 1,
    "id": "dracula",
    "display_name": "Dracula",
    "author": "",
    "license": "",
    "tags": ["community", "dark"],
    "accessibility": "standard",
    "preferred_mode": { "mode": "follow_system" },
    "accent_source": "theme",
    "transition": {
      "duration_ms": 220,
      "easing": "ease_out",
      "cross_fade": true
    }
  },
  "theme": {
    "...": "EditorTheme fields"
  }
}
```

Use `CommunityThemeBundle::validate` after import. Platform frontends should keep OS accent or wallpaper reading outside this crate, then pass the resulting seed color into `AccentPalette`.

## Schema Version Policy

Community theme bundles use `manifest.schema_version` as the compatibility
boundary for JSON import/export. The current version is
`COMMUNITY_THEME_SCHEMA_VERSION = 1`.

Version `1` guarantees:

- `manifest.schema_version`, `id`, and `display_name` are the stable manifest
  identity fields.
- Optional manifest fields use serde defaults so older v1 bundles can omit
  `author`, `license`, `tags`, `accessibility`, `preferred_mode`,
  `accent_source`, or `transition`.
- Bundles that omit `schema_version` are treated as v1 for compatibility with
  early community exports.
- `CommunityThemeBundle::validate()` rejects unsupported future schema
  versions instead of silently importing fields this crate may not understand.

Compatibility rules for future changes:

1. Additive optional fields may stay on schema v1 only when they have safe
   defaults and do not change the meaning of existing fields.
2. Renaming fields, changing field meaning, removing required fields, or
   changing color/transition semantics requires a schema-version bump.
3. A schema-version bump must add compatibility tests for the previous version
   and a migration path before accepting imported community bundles.
4. Exporters should always write the current `schema_version`; importers should
   call `CommunityThemeBundle::from_json()` followed by `validate()` before
   showing or saving a community theme.
