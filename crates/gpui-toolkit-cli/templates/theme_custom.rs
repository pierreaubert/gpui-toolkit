//! Custom editor theme, emitted by `gpui-toolkit template theme-custom`.
//!
//! Start from a built-in preset, override the knobs you care about, then
//! validate: every override below keeps WCAG AA contrast, and the final
//! `validate_accessibility` call proves it. Each section names every
//! `EditorTheme` field it covers as `theme.<field>` so the drift test
//! fails when a new field has no documented home.

use gpui_themes::{BuiltInThemePreset, Color, EditorTheme};

/// Custom dark theme with an indigo accent.
///
/// # Panics
///
/// Panics when the overrides break WCAG AA contrast. That is the
/// point: fail in the theme, not in a user's color picker.
pub fn custom_theme() -> EditorTheme {
    let mut theme = EditorTheme::preset(BuiltInThemePreset::Dark);

    // Identity: shown in pickers and exported bundles.
    theme.name = String::from("Custom Dark");

    // Surfaces: app background ladder plus card/hover/selected states.
    // Untouched siblings: theme.background, theme.background_secondary,
    // theme.background_tertiary, theme.surface, theme.surface_hover,
    // theme.surface_selected.

    // Text: primary must hold 4.5:1 against background and surface (see
    // validation below); the rest are de-emphasized rungs.
    // Untouched siblings: theme.text_primary, theme.text_secondary,
    // theme.text_muted, theme.text_disabled.

    // Borders: resting hairlines plus the focus ring, which tracks accent.
    theme.border_focused = Color::from_hex(0x3b5bdb);
    // Untouched sibling: theme.border.

    // Accent: the interactive color. `text_on_accent` must hold 4.5:1
    // against `accent`; the muted pair backs selected-row washes.
    theme.accent = Color::from_hex(0x3b5bdb);
    theme.accent_hover = Color::from_hex(0x2f4bc4);
    theme.accent_muted = Color::from_hex(0x3b5bdb).with_alpha(0.25);
    theme.text_on_accent = Color::from_hex(0xffffff);
    theme.text_on_accent_muted = Color::from_hex(0xffffff).with_alpha(0.8);

    // Semantic colors: toasts, validation, status dots. Free of contrast
    // checks, but keep them distinguishable on dark surfaces.
    theme.success = Color::from_hex(0x3fb950);
    theme.warning = Color::from_hex(0xd29922);
    theme.error = Color::from_hex(0xf85149);
    theme.info = Color::from_hex(0x58a6ff);

    // Level meters: normal/warning/clip rungs for audio meters.
    theme.meter_normal = Color::from_hex(0x3fb950);
    theme.meter_warning = Color::from_hex(0xd29922);
    theme.meter_clip = Color::from_hex(0xf85149);

    // Mixer buttons: active-state tints for mute, solo, and dim.
    theme.button_mute_active = Color::from_hex(0xf85149);
    theme.button_solo_active = Color::from_hex(0xd29922);
    theme.button_dim_active = Color::from_hex(0x58a6ff);

    // Playback bar: trough plus the fill, which tracks accent.
    theme.progress_bar_bg = Color::from_hex(0x2d2d2d);
    theme.progress_bar_fill = Color::from_hex(0x3b5bdb);

    // Toast backgrounds: per-variant washes behind toast text.
    theme.toast_success_bg = Color::from_hex(0x1a3a24);
    theme.toast_error_bg = Color::from_hex(0x3a1a1a);
    theme.toast_info_bg = Color::from_hex(0x1a2a3a);
    theme.toast_warning_bg = Color::from_hex(0x3a2f1a);

    // Plugin colors: one accent per processor family in graphs and lists.
    theme.plugin_colors.eq = Color::from_hex(0x58a6ff);
    // Untouched sibling struct: theme.plugin_colors (gain, upmixer,
    // compressor, limiter, gate, loudness, binaural, convolution,
    // monitor, spectrum, mute_solo).

    // Graph colors: RoomEQ curve families plus grid and error lines.
    theme.graph_colors.input = Color::from_hex(0x8b949e);
    // Untouched sibling struct: theme.graph_colors (target,
    // filter_response, corrected, error, deviation, grid,
    // secondary_line, directivity_er, directivity_sp).

    // Band colors: per-band tints for EQ band handles, in band order.
    // Untouched sibling: theme.band_colors.

    // EQ curve colors: curve canvas, boost/cut strokes, and fills.
    theme.eq_curve_colors.curve_boost = Color::from_hex(0x3fb950);
    // Untouched sibling struct: theme.eq_curve_colors (background, grid,
    // curve_cut, fill_boost, fill_cut, zero_line).

    // Spectrum colors: analyzer canvas plus per-region magnitudes.
    theme.spectrum_colors.background = Color::from_hex(0x161616);
    // Untouched sibling struct: theme.spectrum_colors (bass, mids, treble).

    // Meter colors: meter canvas, rungs, peak hold, and scale text.
    theme.meter_colors.background = Color::from_hex(0x161616);
    // Untouched sibling struct: theme.meter_colors (normal, warning, clip,
    // peak, text).

    // Indicators: peak hold, drag-and-drop, neutral marker, warning wash,
    // knob face, optimizer highlight, and layout grid lines.
    theme.peak_indicator = Color::from_hex(0xf85149);
    theme.drag_over_highlight = Color::from_hex(0x3b5bdb).with_alpha(0.25);
    theme.drag_over_border = Color::from_hex(0x3b5bdb);
    theme.neutral_indicator = Color::from_hex(0x8b949e);
    theme.warning_background = Color::from_hex(0x3a2f1a);
    theme.knob_color = Color::from_hex(0xc9d1d9);
    theme.optimization_color = Color::from_hex(0x3fb950);
    theme.grid_color = Color::from_hex(0x2d2d2d);

    // Layout and type: separator thickness, UI font stack, and the
    // platform design-language tag (informational; geometry lives in
    // `DesignSystem`).
    theme.separator_size = 1.0;
    theme.font_family = String::from("Inter, system-ui, sans-serif");
    theme.design_language = String::from("neutral");

    // Prove the overrides keep WCAG AA contrast (text on background,
    // text on surface, text on accent). Fails loudly here, not in a
    // user's color picker.
    theme
        .validate_accessibility()
        .expect("custom theme keeps WCAG AA contrast");
    theme
}
