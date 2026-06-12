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
