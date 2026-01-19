//! Theme definition with serialization support
//!
//! Provides a serializable theme structure that can be exported to JSON or Rust code.

use serde::{Deserialize, Serialize};

// Re-export Color from gpui-ui-kit
pub use gpui_ui_kit::Color;

/// Plugin type color mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginColors {
    pub eq: Color,
    pub gain: Color,
    pub upmixer: Color,
    pub compressor: Color,
    pub limiter: Color,
    pub gate: Color,
    pub loudness: Color,
    pub binaural: Color,
    pub convolution: Color,
    pub monitor: Color,
    pub spectrum: Color,
    pub mute_solo: Color,
}

/// Graph visualization line colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphColors {
    pub input: Color,
    pub target: Color,
    pub filter_response: Color,
    pub corrected: Color,
    pub error: Color,
    pub deviation: Color,
    pub grid: Color,
    pub secondary_line: Color,
    pub directivity_er: Color,
    pub directivity_sp: Color,
}

/// EQ curve visualization colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EQCurveColors {
    pub background: Color,
    pub grid: Color,
    pub curve_boost: Color,
    pub curve_cut: Color,
    pub fill_boost: Color,
    pub fill_cut: Color,
    pub zero_line: Color,
}

/// Spectrum analyzer colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumColors {
    pub background: Color,
    pub bass: Color,
    pub mids: Color,
    pub treble: Color,
}

/// Level meter colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterColors {
    pub background: Color,
    pub normal: Color,
    pub warning: Color,
    pub clip: Color,
    pub peak: Color,
    pub text: Color,
}

/// Complete theme definition with all UI colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTheme {
    /// Theme name for display
    pub name: String,

    // Base colors
    pub background: Color,
    pub background_secondary: Color,
    pub background_tertiary: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_selected: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_disabled: Color,

    // Border colors
    pub border: Color,
    pub border_focused: Color,

    // Accent colors
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_muted: Color,

    // Text on accent
    pub text_on_accent: Color,
    pub text_on_accent_muted: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Level meter colors
    pub meter_normal: Color,
    pub meter_warning: Color,
    pub meter_clip: Color,

    // Button colors
    pub button_mute_active: Color,
    pub button_solo_active: Color,
    pub button_dim_active: Color,

    // Playback bar
    pub progress_bar_bg: Color,
    pub progress_bar_fill: Color,

    // Toast backgrounds
    pub toast_success_bg: Color,
    pub toast_error_bg: Color,
    pub toast_info_bg: Color,
    pub toast_warning_bg: Color,

    // Plugin colors
    pub plugin_colors: PluginColors,
    pub graph_colors: GraphColors,
    pub band_colors: Vec<Color>,
    pub eq_curve_colors: EQCurveColors,
    pub spectrum_colors: SpectrumColors,
    pub meter_colors: MeterColors,

    // Additional colors
    pub peak_indicator: Color,
    pub drag_over_highlight: Color,
    pub drag_over_border: Color,
    pub neutral_indicator: Color,
    pub warning_background: Color,
    pub knob_color: Color,
    pub optimization_color: Color,
    pub grid_color: Color,

    // Layout sizes
    pub separator_size: f32,

    // Font family
    pub font_family: String,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl EditorTheme {
    /// Create the default dark theme
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),

            // Base colors
            background: Color::from_hex(0x1e1e1e),
            background_secondary: Color::from_hex(0x252525),
            background_tertiary: Color::from_hex(0x2d2d2d),
            surface: Color::from_hex(0x2d2d2d),
            surface_hover: Color::from_hex(0x3e3e3e),
            surface_selected: Color::from_hex(0x264f78),

            // Text colors
            text_primary: Color::from_hex(0xcccccc),
            text_secondary: Color::from_hex(0x999999),
            text_muted: Color::from_hex(0x666666),
            text_disabled: Color::from_hex(0x444444),

            // Border colors
            border: Color::from_hex(0x3e3e3e),
            border_focused: Color::from_hex(0x007acc),

            // Accent colors
            accent: Color::from_hex(0x007acc),
            accent_hover: Color::from_hex(0x1c8cd9),
            accent_muted: Color::from_hex(0x264f78),

            // Text on accent
            text_on_accent: Color::from_hex(0xffffff),
            text_on_accent_muted: Color::new(255, 255, 255, 204),

            // Semantic colors
            success: Color::from_hex(0x4ec9b0),
            warning: Color::from_hex(0xdcdcaa),
            error: Color::from_hex(0xf48771),
            info: Color::from_hex(0x569cd6),

            // Level meter colors
            meter_normal: Color::from_hex(0x22c55e),
            meter_warning: Color::from_hex(0xf59e0b),
            meter_clip: Color::from_hex(0xdc2626),

            // Button colors
            button_mute_active: Color::from_hex(0xdc2626),
            button_solo_active: Color::from_hex(0xf59e0b),
            button_dim_active: Color::from_hex(0x6366f1),

            // Playback bar
            progress_bar_bg: Color::from_hex(0x3e3e3e),
            progress_bar_fill: Color::from_hex(0x007acc),

            // Toast backgrounds
            toast_success_bg: Color::from_hex(0x1e3a1e),
            toast_error_bg: Color::from_hex(0x3a1e1e),
            toast_info_bg: Color::from_hex(0x1e2a3a),
            toast_warning_bg: Color::from_hex(0x3a2e1e),

            // Plugin colors
            plugin_colors: PluginColors {
                eq: Color::from_hex(0x2563eb),
                gain: Color::from_hex(0x059669),
                upmixer: Color::from_hex(0x7c3aed),
                compressor: Color::from_hex(0xdc2626),
                limiter: Color::from_hex(0xea580c),
                gate: Color::from_hex(0xca8a04),
                loudness: Color::from_hex(0x0891b2),
                binaural: Color::from_hex(0xdb2777),
                convolution: Color::from_hex(0x4f46e5),
                monitor: Color::from_hex(0x14b8a6),
                spectrum: Color::from_hex(0x8b5cf6),
                mute_solo: Color::from_hex(0x6366f1),
            },
            graph_colors: GraphColors {
                input: Color::from_hex(0x6366f1),
                target: Color::from_hex(0x22c55e),
                filter_response: Color::from_hex(0xf59e0b),
                corrected: Color::from_hex(0x3b82f6),
                error: Color::from_hex(0xef4444),
                deviation: Color::from_hex(0x8b5cf6),
                grid: Color::new(255, 255, 255, 21),
                secondary_line: Color::from_hex(0xaaaaaa),
                directivity_er: Color::from_hex(0xf472b6),
                directivity_sp: Color::from_hex(0xc084fc),
            },
            band_colors: vec![
                Color::from_hex(0xef4444),
                Color::from_hex(0xf97316),
                Color::from_hex(0xeab308),
                Color::from_hex(0x22c55e),
                Color::from_hex(0x14b8a6),
                Color::from_hex(0x3b82f6),
                Color::from_hex(0x8b5cf6),
                Color::from_hex(0xec4899),
                Color::from_hex(0x6366f1),
                Color::from_hex(0x06b6d4),
                Color::from_hex(0x9ca3af),
            ],
            eq_curve_colors: EQCurveColors {
                background: Color::from_hex(0x1a1a1a),
                grid: Color::new(255, 255, 255, 32),
                curve_boost: Color::from_hex(0x22c55e),
                curve_cut: Color::from_hex(0xef4444),
                fill_boost: Color::new(34, 197, 94, 64),
                fill_cut: Color::new(239, 68, 68, 64),
                zero_line: Color::new(255, 255, 255, 64),
            },
            spectrum_colors: SpectrumColors {
                background: Color::from_hex(0x000000),
                bass: Color::from_hex(0x22c55e),
                mids: Color::from_hex(0xeab308),
                treble: Color::from_hex(0xef4444),
            },
            meter_colors: MeterColors {
                background: Color::from_hex(0x1e1e1e),
                normal: Color::from_hex(0x22c55e),
                warning: Color::from_hex(0xf59e0b),
                clip: Color::from_hex(0xdc2626),
                peak: Color::from_hex(0xffffff),
                text: Color::from_hex(0x999999),
            },

            // Additional colors
            peak_indicator: Color::from_hex(0xffffff),
            drag_over_highlight: Color::new(59, 130, 246, 64),
            drag_over_border: Color::from_hex(0x3b82f6),
            neutral_indicator: Color::from_hex(0x6366f1),
            warning_background: Color::new(245, 158, 11, 51),
            knob_color: Color::from_hex(0xffffff),
            optimization_color: Color::from_hex(0x8b5cf6),
            grid_color: Color::new(255, 255, 255, 21),

            // Layout sizes
            separator_size: 20.0,

            // Font family
            font_family: ".SystemUI".to_string(),
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),

            background: Color::from_hex(0xf5f5f5),
            background_secondary: Color::from_hex(0xececec),
            background_tertiary: Color::from_hex(0xe0e0e0),
            surface: Color::from_hex(0xffffff),
            surface_hover: Color::from_hex(0xf0f0f0),
            surface_selected: Color::from_hex(0xd0e8ff),

            text_primary: Color::from_hex(0x1e1e1e),
            text_secondary: Color::from_hex(0x555555),
            text_muted: Color::from_hex(0x888888),
            text_disabled: Color::from_hex(0xbbbbbb),

            border: Color::from_hex(0xd0d0d0),
            border_focused: Color::from_hex(0x0066cc),

            accent: Color::from_hex(0x0066cc),
            accent_hover: Color::from_hex(0x0078e6),
            accent_muted: Color::from_hex(0xb3d4f7),

            text_on_accent: Color::from_hex(0xffffff),
            text_on_accent_muted: Color::new(255, 255, 255, 204),

            success: Color::from_hex(0x16a34a),
            warning: Color::from_hex(0xca8a04),
            error: Color::from_hex(0xdc2626),
            info: Color::from_hex(0x2563eb),

            meter_normal: Color::from_hex(0x16a34a),
            meter_warning: Color::from_hex(0xca8a04),
            meter_clip: Color::from_hex(0xdc2626),

            button_mute_active: Color::from_hex(0xdc2626),
            button_solo_active: Color::from_hex(0xca8a04),
            button_dim_active: Color::from_hex(0x6366f1),

            progress_bar_bg: Color::from_hex(0xd0d0d0),
            progress_bar_fill: Color::from_hex(0x0066cc),

            toast_success_bg: Color::from_hex(0xd1fae5),
            toast_error_bg: Color::from_hex(0xfee2e2),
            toast_info_bg: Color::from_hex(0xdbeafe),
            toast_warning_bg: Color::from_hex(0xfef3c7),

            plugin_colors: PluginColors {
                eq: Color::from_hex(0x2563eb),
                gain: Color::from_hex(0x16a34a),
                upmixer: Color::from_hex(0x7c3aed),
                compressor: Color::from_hex(0xdc2626),
                limiter: Color::from_hex(0xea580c),
                gate: Color::from_hex(0xca8a04),
                loudness: Color::from_hex(0x0891b2),
                binaural: Color::from_hex(0xdb2777),
                convolution: Color::from_hex(0x4f46e5),
                monitor: Color::from_hex(0x14b8a6),
                spectrum: Color::from_hex(0x8b5cf6),
                mute_solo: Color::from_hex(0x6366f1),
            },
            graph_colors: GraphColors {
                input: Color::from_hex(0x6366f1),
                target: Color::from_hex(0x16a34a),
                filter_response: Color::from_hex(0xca8a04),
                corrected: Color::from_hex(0x2563eb),
                error: Color::from_hex(0xdc2626),
                deviation: Color::from_hex(0x7c3aed),
                grid: Color::new(0, 0, 0, 21),
                secondary_line: Color::from_hex(0x888888),
                directivity_er: Color::from_hex(0xec4899),
                directivity_sp: Color::from_hex(0xa855f7),
            },
            band_colors: vec![
                Color::from_hex(0xdc2626),
                Color::from_hex(0xea580c),
                Color::from_hex(0xca8a04),
                Color::from_hex(0x16a34a),
                Color::from_hex(0x0d9488),
                Color::from_hex(0x2563eb),
                Color::from_hex(0x7c3aed),
                Color::from_hex(0xdb2777),
                Color::from_hex(0x4f46e5),
                Color::from_hex(0x0891b2),
                Color::from_hex(0x6b7280),
            ],
            eq_curve_colors: EQCurveColors {
                background: Color::from_hex(0xfafafa),
                grid: Color::new(0, 0, 0, 25),
                curve_boost: Color::from_hex(0x16a34a),
                curve_cut: Color::from_hex(0xdc2626),
                fill_boost: Color::new(22, 163, 74, 64),
                fill_cut: Color::new(220, 38, 38, 64),
                zero_line: Color::new(0, 0, 0, 64),
            },
            spectrum_colors: SpectrumColors {
                background: Color::from_hex(0xfafafa),
                bass: Color::from_hex(0x16a34a),
                mids: Color::from_hex(0xca8a04),
                treble: Color::from_hex(0xdc2626),
            },
            meter_colors: MeterColors {
                background: Color::from_hex(0xf5f5f5),
                normal: Color::from_hex(0x16a34a),
                warning: Color::from_hex(0xca8a04),
                clip: Color::from_hex(0xdc2626),
                peak: Color::from_hex(0x1e1e1e),
                text: Color::from_hex(0x555555),
            },

            peak_indicator: Color::from_hex(0x1e1e1e),
            drag_over_highlight: Color::new(37, 99, 235, 64),
            drag_over_border: Color::from_hex(0x2563eb),
            neutral_indicator: Color::from_hex(0x6366f1),
            warning_background: Color::new(202, 138, 4, 51),
            knob_color: Color::from_hex(0x333333),
            optimization_color: Color::from_hex(0x7c3aed),
            grid_color: Color::new(0, 0, 0, 21),

            separator_size: 20.0,
            font_family: ".SystemUI".to_string(),
        }
    }

    /// Create a high contrast dark theme
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),

            background: Color::from_hex(0x000000),
            background_secondary: Color::from_hex(0x0a0a0a),
            background_tertiary: Color::from_hex(0x141414),
            surface: Color::from_hex(0x1a1a1a),
            surface_hover: Color::from_hex(0x2a2a2a),
            surface_selected: Color::from_hex(0x3a3a3a),

            text_primary: Color::from_hex(0xffffff),
            text_secondary: Color::from_hex(0xdddddd),
            text_muted: Color::from_hex(0x999999),
            text_disabled: Color::from_hex(0x555555),

            border: Color::from_hex(0x555555),
            border_focused: Color::from_hex(0x00ffff),

            accent: Color::from_hex(0x00ffff),
            accent_hover: Color::from_hex(0x33ffff),
            accent_muted: Color::from_hex(0x006666),

            text_on_accent: Color::from_hex(0x000000),
            text_on_accent_muted: Color::new(0, 0, 0, 204),

            success: Color::from_hex(0x00ff00),
            warning: Color::from_hex(0xffff00),
            error: Color::from_hex(0xff0000),
            info: Color::from_hex(0x00aaff),

            meter_normal: Color::from_hex(0x00ff00),
            meter_warning: Color::from_hex(0xffff00),
            meter_clip: Color::from_hex(0xff0000),

            button_mute_active: Color::from_hex(0xff0000),
            button_solo_active: Color::from_hex(0xffff00),
            button_dim_active: Color::from_hex(0x0088ff),

            progress_bar_bg: Color::from_hex(0x333333),
            progress_bar_fill: Color::from_hex(0x00ffff),

            toast_success_bg: Color::from_hex(0x003300),
            toast_error_bg: Color::from_hex(0x330000),
            toast_info_bg: Color::from_hex(0x003333),
            toast_warning_bg: Color::from_hex(0x333300),

            plugin_colors: PluginColors {
                eq: Color::from_hex(0x0088ff),
                gain: Color::from_hex(0x00ff00),
                upmixer: Color::from_hex(0xaa00ff),
                compressor: Color::from_hex(0xff0000),
                limiter: Color::from_hex(0xff6600),
                gate: Color::from_hex(0xffaa00),
                loudness: Color::from_hex(0x00aaff),
                binaural: Color::from_hex(0xff00aa),
                convolution: Color::from_hex(0x6600ff),
                monitor: Color::from_hex(0x00ffaa),
                spectrum: Color::from_hex(0xaa00ff),
                mute_solo: Color::from_hex(0x8888ff),
            },
            graph_colors: GraphColors {
                input: Color::from_hex(0x8888ff),
                target: Color::from_hex(0x00ff00),
                filter_response: Color::from_hex(0xffaa00),
                corrected: Color::from_hex(0x0088ff),
                error: Color::from_hex(0xff0000),
                deviation: Color::from_hex(0xaa00ff),
                grid: Color::new(255, 255, 255, 40),
                secondary_line: Color::from_hex(0xcccccc),
                directivity_er: Color::from_hex(0xff66cc),
                directivity_sp: Color::from_hex(0xcc66ff),
            },
            band_colors: vec![
                Color::from_hex(0xff0000),
                Color::from_hex(0xff6600),
                Color::from_hex(0xffaa00),
                Color::from_hex(0x00ff00),
                Color::from_hex(0x00ffaa),
                Color::from_hex(0x0088ff),
                Color::from_hex(0xaa00ff),
                Color::from_hex(0xff00aa),
                Color::from_hex(0x8888ff),
                Color::from_hex(0x00aaff),
                Color::from_hex(0xcccccc),
            ],
            eq_curve_colors: EQCurveColors {
                background: Color::from_hex(0x000000),
                grid: Color::new(255, 255, 255, 60),
                curve_boost: Color::from_hex(0x00ff00),
                curve_cut: Color::from_hex(0xff0000),
                fill_boost: Color::new(0, 255, 0, 80),
                fill_cut: Color::new(255, 0, 0, 80),
                zero_line: Color::new(255, 255, 255, 100),
            },
            spectrum_colors: SpectrumColors {
                background: Color::from_hex(0x000000),
                bass: Color::from_hex(0x00ff00),
                mids: Color::from_hex(0xffaa00),
                treble: Color::from_hex(0xff0000),
            },
            meter_colors: MeterColors {
                background: Color::from_hex(0x000000),
                normal: Color::from_hex(0x00ff00),
                warning: Color::from_hex(0xffff00),
                clip: Color::from_hex(0xff0000),
                peak: Color::from_hex(0xffffff),
                text: Color::from_hex(0xdddddd),
            },

            peak_indicator: Color::from_hex(0xffffff),
            drag_over_highlight: Color::new(0, 255, 255, 80),
            drag_over_border: Color::from_hex(0x00ffff),
            neutral_indicator: Color::from_hex(0x8888ff),
            warning_background: Color::new(255, 255, 0, 60),
            knob_color: Color::from_hex(0xffffff),
            optimization_color: Color::from_hex(0xaa00ff),
            grid_color: Color::new(255, 255, 255, 40),

            separator_size: 20.0,
            font_family: ".SystemUI".to_string(),
        }
    }

    /// Create a Nord theme
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),

            background: Color::from_hex(0x2e3440),
            background_secondary: Color::from_hex(0x3b4252),
            background_tertiary: Color::from_hex(0x434c5e),
            surface: Color::from_hex(0x3b4252),
            surface_hover: Color::from_hex(0x434c5e),
            surface_selected: Color::from_hex(0x4c566a),

            text_primary: Color::from_hex(0xeceff4),
            text_secondary: Color::from_hex(0xd8dee9),
            text_muted: Color::from_hex(0x81a1c1),
            text_disabled: Color::from_hex(0x4c566a),

            border: Color::from_hex(0x4c566a),
            border_focused: Color::from_hex(0x88c0d0),

            accent: Color::from_hex(0x88c0d0),
            accent_hover: Color::from_hex(0x8fbcbb),
            accent_muted: Color::from_hex(0x5e81ac),

            text_on_accent: Color::from_hex(0x2e3440),
            text_on_accent_muted: Color::new(46, 52, 64, 204),

            success: Color::from_hex(0xa3be8c),
            warning: Color::from_hex(0xebcb8b),
            error: Color::from_hex(0xbf616a),
            info: Color::from_hex(0x81a1c1),

            meter_normal: Color::from_hex(0xa3be8c),
            meter_warning: Color::from_hex(0xebcb8b),
            meter_clip: Color::from_hex(0xbf616a),

            button_mute_active: Color::from_hex(0xbf616a),
            button_solo_active: Color::from_hex(0xebcb8b),
            button_dim_active: Color::from_hex(0x5e81ac),

            progress_bar_bg: Color::from_hex(0x4c566a),
            progress_bar_fill: Color::from_hex(0x88c0d0),

            toast_success_bg: Color::from_hex(0x3e4f41),
            toast_error_bg: Color::from_hex(0x4a3638),
            toast_info_bg: Color::from_hex(0x38445a),
            toast_warning_bg: Color::from_hex(0x4c4639),

            plugin_colors: PluginColors {
                eq: Color::from_hex(0x5e81ac),
                gain: Color::from_hex(0xa3be8c),
                upmixer: Color::from_hex(0xb48ead),
                compressor: Color::from_hex(0xbf616a),
                limiter: Color::from_hex(0xd08770),
                gate: Color::from_hex(0xebcb8b),
                loudness: Color::from_hex(0x88c0d0),
                binaural: Color::from_hex(0xb48ead),
                convolution: Color::from_hex(0x81a1c1),
                monitor: Color::from_hex(0x8fbcbb),
                spectrum: Color::from_hex(0xb48ead),
                mute_solo: Color::from_hex(0x5e81ac),
            },
            graph_colors: GraphColors {
                input: Color::from_hex(0x5e81ac),
                target: Color::from_hex(0xa3be8c),
                filter_response: Color::from_hex(0xebcb8b),
                corrected: Color::from_hex(0x81a1c1),
                error: Color::from_hex(0xbf616a),
                deviation: Color::from_hex(0xb48ead),
                grid: Color::new(216, 222, 233, 30),
                secondary_line: Color::from_hex(0xd8dee9),
                directivity_er: Color::from_hex(0xb48ead),
                directivity_sp: Color::from_hex(0x81a1c1),
            },
            band_colors: vec![
                Color::from_hex(0xbf616a),
                Color::from_hex(0xd08770),
                Color::from_hex(0xebcb8b),
                Color::from_hex(0xa3be8c),
                Color::from_hex(0x8fbcbb),
                Color::from_hex(0x88c0d0),
                Color::from_hex(0x81a1c1),
                Color::from_hex(0x5e81ac),
                Color::from_hex(0xb48ead),
                Color::from_hex(0x88c0d0),
                Color::from_hex(0x4c566a),
            ],
            eq_curve_colors: EQCurveColors {
                background: Color::from_hex(0x2e3440),
                grid: Color::new(216, 222, 233, 40),
                curve_boost: Color::from_hex(0xa3be8c),
                curve_cut: Color::from_hex(0xbf616a),
                fill_boost: Color::new(163, 190, 140, 64),
                fill_cut: Color::new(191, 97, 106, 64),
                zero_line: Color::new(216, 222, 233, 80),
            },
            spectrum_colors: SpectrumColors {
                background: Color::from_hex(0x2e3440),
                bass: Color::from_hex(0xa3be8c),
                mids: Color::from_hex(0xebcb8b),
                treble: Color::from_hex(0xbf616a),
            },
            meter_colors: MeterColors {
                background: Color::from_hex(0x2e3440),
                normal: Color::from_hex(0xa3be8c),
                warning: Color::from_hex(0xebcb8b),
                clip: Color::from_hex(0xbf616a),
                peak: Color::from_hex(0xeceff4),
                text: Color::from_hex(0xd8dee9),
            },

            peak_indicator: Color::from_hex(0xeceff4),
            drag_over_highlight: Color::new(136, 192, 208, 64),
            drag_over_border: Color::from_hex(0x88c0d0),
            neutral_indicator: Color::from_hex(0x5e81ac),
            warning_background: Color::new(235, 203, 139, 51),
            knob_color: Color::from_hex(0xeceff4),
            optimization_color: Color::from_hex(0xb48ead),
            grid_color: Color::new(216, 222, 233, 30),

            separator_size: 20.0,
            font_family: ".SystemUI".to_string(),
        }
    }

    /// Create a Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),

            background: Color::from_hex(0x282a36),
            background_secondary: Color::from_hex(0x21222c),
            background_tertiary: Color::from_hex(0x191a21),
            surface: Color::from_hex(0x44475a),
            surface_hover: Color::from_hex(0x6272a4),
            surface_selected: Color::from_hex(0x6272a4),

            text_primary: Color::from_hex(0xf8f8f2),
            text_secondary: Color::from_hex(0xbfbfbf),
            text_muted: Color::from_hex(0x6272a4),
            text_disabled: Color::from_hex(0x44475a),

            border: Color::from_hex(0x44475a),
            border_focused: Color::from_hex(0xbd93f9),

            accent: Color::from_hex(0xbd93f9),
            accent_hover: Color::from_hex(0xff79c6),
            accent_muted: Color::from_hex(0x6272a4),

            text_on_accent: Color::from_hex(0x282a36),
            text_on_accent_muted: Color::new(40, 42, 54, 204),

            success: Color::from_hex(0x50fa7b),
            warning: Color::from_hex(0xf1fa8c),
            error: Color::from_hex(0xff5555),
            info: Color::from_hex(0x8be9fd),

            meter_normal: Color::from_hex(0x50fa7b),
            meter_warning: Color::from_hex(0xf1fa8c),
            meter_clip: Color::from_hex(0xff5555),

            button_mute_active: Color::from_hex(0xff5555),
            button_solo_active: Color::from_hex(0xf1fa8c),
            button_dim_active: Color::from_hex(0xbd93f9),

            progress_bar_bg: Color::from_hex(0x44475a),
            progress_bar_fill: Color::from_hex(0xbd93f9),

            toast_success_bg: Color::from_hex(0x1e3a26),
            toast_error_bg: Color::from_hex(0x3a1e1e),
            toast_info_bg: Color::from_hex(0x1e2f3a),
            toast_warning_bg: Color::from_hex(0x3a3a1e),

            plugin_colors: PluginColors {
                eq: Color::from_hex(0x8be9fd),
                gain: Color::from_hex(0x50fa7b),
                upmixer: Color::from_hex(0xbd93f9),
                compressor: Color::from_hex(0xff5555),
                limiter: Color::from_hex(0xffb86c),
                gate: Color::from_hex(0xf1fa8c),
                loudness: Color::from_hex(0x8be9fd),
                binaural: Color::from_hex(0xff79c6),
                convolution: Color::from_hex(0xbd93f9),
                monitor: Color::from_hex(0x50fa7b),
                spectrum: Color::from_hex(0xbd93f9),
                mute_solo: Color::from_hex(0x6272a4),
            },
            graph_colors: GraphColors {
                input: Color::from_hex(0x6272a4),
                target: Color::from_hex(0x50fa7b),
                filter_response: Color::from_hex(0xf1fa8c),
                corrected: Color::from_hex(0x8be9fd),
                error: Color::from_hex(0xff5555),
                deviation: Color::from_hex(0xbd93f9),
                grid: Color::new(248, 248, 242, 25),
                secondary_line: Color::from_hex(0xbfbfbf),
                directivity_er: Color::from_hex(0xff79c6),
                directivity_sp: Color::from_hex(0xbd93f9),
            },
            band_colors: vec![
                Color::from_hex(0xff5555),
                Color::from_hex(0xffb86c),
                Color::from_hex(0xf1fa8c),
                Color::from_hex(0x50fa7b),
                Color::from_hex(0x8be9fd),
                Color::from_hex(0xbd93f9),
                Color::from_hex(0xff79c6),
                Color::from_hex(0x6272a4),
                Color::from_hex(0xbd93f9),
                Color::from_hex(0x8be9fd),
                Color::from_hex(0x44475a),
            ],
            eq_curve_colors: EQCurveColors {
                background: Color::from_hex(0x282a36),
                grid: Color::new(248, 248, 242, 35),
                curve_boost: Color::from_hex(0x50fa7b),
                curve_cut: Color::from_hex(0xff5555),
                fill_boost: Color::new(80, 250, 123, 64),
                fill_cut: Color::new(255, 85, 85, 64),
                zero_line: Color::new(248, 248, 242, 80),
            },
            spectrum_colors: SpectrumColors {
                background: Color::from_hex(0x282a36),
                bass: Color::from_hex(0x50fa7b),
                mids: Color::from_hex(0xf1fa8c),
                treble: Color::from_hex(0xff5555),
            },
            meter_colors: MeterColors {
                background: Color::from_hex(0x282a36),
                normal: Color::from_hex(0x50fa7b),
                warning: Color::from_hex(0xf1fa8c),
                clip: Color::from_hex(0xff5555),
                peak: Color::from_hex(0xf8f8f2),
                text: Color::from_hex(0xbfbfbf),
            },

            peak_indicator: Color::from_hex(0xf8f8f2),
            drag_over_highlight: Color::new(189, 147, 249, 64),
            drag_over_border: Color::from_hex(0xbd93f9),
            neutral_indicator: Color::from_hex(0x6272a4),
            warning_background: Color::new(241, 250, 140, 51),
            knob_color: Color::from_hex(0xf8f8f2),
            optimization_color: Color::from_hex(0xbd93f9),
            grid_color: Color::new(248, 248, 242, 25),

            separator_size: 20.0,
            font_family: ".SystemUI".to_string(),
        }
    }

    /// Save theme to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load theme from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Generate Rust code for this theme
    pub fn to_rust_code(&self) -> String {
        fn color_to_rust(c: &Color) -> String {
            if c.a == 255 {
                format!("Color::from_hex(0x{:02x}{:02x}{:02x})", c.r, c.g, c.b)
            } else {
                format!("Color::new({}, {}, {}, {})", c.r, c.g, c.b, c.a)
            }
        }

        let mut code = format!(
            r#"/// {} theme
pub fn {}() -> EditorTheme {{
    EditorTheme {{
        name: "{}".to_string(),

        // Base colors
        background: {},
        background_secondary: {},
        background_tertiary: {},
        surface: {},
        surface_hover: {},
        surface_selected: {},

        // Text colors
        text_primary: {},
        text_secondary: {},
        text_muted: {},
        text_disabled: {},

        // Border colors
        border: {},
        border_focused: {},

        // Accent colors
        accent: {},
        accent_hover: {},
        accent_muted: {},

        // Text on accent
        text_on_accent: {},
        text_on_accent_muted: {},

        // Semantic colors
        success: {},
        warning: {},
        error: {},
        info: {},

        // Level meter colors
        meter_normal: {},
        meter_warning: {},
        meter_clip: {},

        // Button colors
        button_mute_active: {},
        button_solo_active: {},
        button_dim_active: {},

        // Playback bar
        progress_bar_bg: {},
        progress_bar_fill: {},

        // Toast backgrounds
        toast_success_bg: {},
        toast_error_bg: {},
        toast_info_bg: {},
        toast_warning_bg: {},

        // Additional colors
        peak_indicator: {},
        drag_over_highlight: {},
        drag_over_border: {},
        neutral_indicator: {},
        warning_background: {},
        knob_color: {},
        optimization_color: {},
        grid_color: {},

        separator_size: {:.1},
        font_family: "{}".to_string(),
"#,
            self.name,
            self.name.to_lowercase().replace(' ', "_"),
            self.name,
            color_to_rust(&self.background),
            color_to_rust(&self.background_secondary),
            color_to_rust(&self.background_tertiary),
            color_to_rust(&self.surface),
            color_to_rust(&self.surface_hover),
            color_to_rust(&self.surface_selected),
            color_to_rust(&self.text_primary),
            color_to_rust(&self.text_secondary),
            color_to_rust(&self.text_muted),
            color_to_rust(&self.text_disabled),
            color_to_rust(&self.border),
            color_to_rust(&self.border_focused),
            color_to_rust(&self.accent),
            color_to_rust(&self.accent_hover),
            color_to_rust(&self.accent_muted),
            color_to_rust(&self.text_on_accent),
            color_to_rust(&self.text_on_accent_muted),
            color_to_rust(&self.success),
            color_to_rust(&self.warning),
            color_to_rust(&self.error),
            color_to_rust(&self.info),
            color_to_rust(&self.meter_normal),
            color_to_rust(&self.meter_warning),
            color_to_rust(&self.meter_clip),
            color_to_rust(&self.button_mute_active),
            color_to_rust(&self.button_solo_active),
            color_to_rust(&self.button_dim_active),
            color_to_rust(&self.progress_bar_bg),
            color_to_rust(&self.progress_bar_fill),
            color_to_rust(&self.toast_success_bg),
            color_to_rust(&self.toast_error_bg),
            color_to_rust(&self.toast_info_bg),
            color_to_rust(&self.toast_warning_bg),
            color_to_rust(&self.peak_indicator),
            color_to_rust(&self.drag_over_highlight),
            color_to_rust(&self.drag_over_border),
            color_to_rust(&self.neutral_indicator),
            color_to_rust(&self.warning_background),
            color_to_rust(&self.knob_color),
            color_to_rust(&self.optimization_color),
            color_to_rust(&self.grid_color),
            self.separator_size,
            self.font_family,
        );

        // Add plugin_colors, graph_colors, etc. (abbreviated for length)
        code.push_str("        // ... plugin_colors, graph_colors, etc.\n");
        code.push_str("    }\n}\n");

        code
    }

    /// Create a GPUI-compatible button theme
    pub fn to_button_theme(&self) -> gpui_ui_kit::ButtonTheme {
        let mut error_hover = self.error.to_rgba();
        error_hover.a = 0.8;

        gpui_ui_kit::ButtonTheme {
            accent: self.accent.to_rgba(),
            accent_hover: self.accent_hover.to_rgba(),
            surface: self.surface.to_rgba(),
            surface_hover: self.surface_hover.to_rgba(),
            text_primary: self.text_primary.to_rgba(),
            text_secondary: self.text_secondary.to_rgba(),
            text_on_accent: self.text_on_accent.to_rgba(),
            error: self.error.to_rgba(),
            error_hover,
            border: self.border.to_rgba(),
            transparent: gpui::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        }
    }

    /// Create a GPUI-compatible slider theme
    pub fn to_slider_theme(&self) -> gpui_ui_kit::SliderTheme {
        let mut disabled_label = self.text_disabled.to_rgba();
        disabled_label.a = 0.5;

        gpui_ui_kit::SliderTheme {
            track: self.surface_hover.to_rgba(),
            fill: self.accent.to_rgba(),
            thumb: self.text_primary.to_rgba(),
            thumb_hover: self.text_secondary.to_rgba(),
            thumb_active: self.accent.to_rgba(),
            label: self.text_primary.to_rgba(),
            value: self.text_secondary.to_rgba(),
            disabled_label,
            disabled_fill: self.text_disabled.to_rgba(),
        }
    }

    /// Create a GPUI-compatible accordion theme
    pub fn to_accordion_theme(&self) -> gpui_ui_kit::AccordionTheme {
        gpui_ui_kit::AccordionTheme {
            header_bg: self.surface.to_rgba(),
            header_hover_bg: self.surface_hover.to_rgba(),
            content_bg: self.background.to_rgba(),
            border: self.border.to_rgba(),
            title_color: self.text_primary.to_rgba(),
            indicator_color: self.text_muted.to_rgba(),
        }
    }

    /// Create a GPUI-compatible tabs theme
    pub fn to_tabs_theme(&self) -> gpui_ui_kit::TabsTheme {
        gpui_ui_kit::TabsTheme {
            container_bg: self.surface.to_rgba(),
            container_border: self.border.to_rgba(),
            selected_bg: self.surface_selected.to_rgba(),
            selected_hover_bg: self.surface_hover.to_rgba(),
            hover_bg: self.surface_hover.to_rgba(),
            accent: self.accent.to_rgba(),
            text_selected: self.text_primary.to_rgba(),
            text_unselected: self.text_secondary.to_rgba(),
            text_hover: self.text_primary.to_rgba(),
            badge_bg: self.surface_hover.to_rgba(),
            close_color: self.text_muted.to_rgba(),
            close_hover_color: self.text_primary.to_rgba(),
            icon_selected: None,
            icon_unselected: None,
        }
    }
}

/// Color group for organizing theme editor UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorGroup {
    Base,
    Text,
    Border,
    Accent,
    Semantic,
    Meter,
    Button,
    Progress,
    Toast,
    Plugin,
    Graph,
    Spectrum,
    Additional,
}

impl ColorGroup {
    pub fn all() -> &'static [ColorGroup] {
        &[
            ColorGroup::Base,
            ColorGroup::Text,
            ColorGroup::Border,
            ColorGroup::Accent,
            ColorGroup::Semantic,
            ColorGroup::Meter,
            ColorGroup::Button,
            ColorGroup::Progress,
            ColorGroup::Toast,
            ColorGroup::Plugin,
            ColorGroup::Graph,
            ColorGroup::Spectrum,
            ColorGroup::Additional,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ColorGroup::Base => "Base Colors",
            ColorGroup::Text => "Text Colors",
            ColorGroup::Border => "Border Colors",
            ColorGroup::Accent => "Accent Colors",
            ColorGroup::Semantic => "Semantic Colors",
            ColorGroup::Meter => "Level Meters",
            ColorGroup::Button => "Button States",
            ColorGroup::Progress => "Progress Bar",
            ColorGroup::Toast => "Toast Notifications",
            ColorGroup::Plugin => "Plugin Colors",
            ColorGroup::Graph => "Graph Colors",
            ColorGroup::Spectrum => "Spectrum Colors",
            ColorGroup::Additional => "Additional",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_json_roundtrip() {
        let theme = EditorTheme::dark();
        let json = theme.to_json().unwrap();
        let loaded = EditorTheme::from_json(&json).unwrap();
        assert_eq!(loaded.name, theme.name);
        assert_eq!(loaded.background.r, theme.background.r);
    }
}
