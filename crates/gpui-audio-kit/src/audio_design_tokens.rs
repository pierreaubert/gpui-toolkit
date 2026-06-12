//! Audio Design Tokens
//!
//! Lightweight design token struct consumed by audio UI components
//! (Potentiometer, VerticalSlider, Toggle). This replaces hardcoded
//! geometry/timing values with configurable parameters driven by the
//! platform design system (Apple HIG, Material 3, Fluent, Neutral).
//!
//! Components use `Default::default()` when no tokens are provided,
//! which reproduces the current hardcoded behavior (backward compatible).
//!
//! The host application converts its `DesignSystem` into `AudioDesignTokens`
//! and passes them to components via `.design_tokens(tokens)` builder methods.

/// Audio component design tokens — configurable geometry and timing.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioDesignTokens {
    // -- Potentiometer (knob) geometry --
    /// Knob arc start angle in degrees from 12 o'clock, clockwise.
    /// Default: 135.0 (7:30 position).
    pub knob_arc_start_deg: f32,
    /// Knob arc sweep in degrees. Dead zone = 360 - sweep.
    /// Default: 270.0 (90° dead zone at bottom).
    pub knob_arc_sweep_deg: f32,
    /// Arc thickness in px (per size variant: [Xs, Sm, Md, Lg]).
    /// Default: [2.5, 3.0, 3.5, 4.0].
    pub knob_arc_widths: [f32; 4],
    /// Track (unfilled) arc thickness in px (per size variant).
    /// When `0.0`, the track arc is hidden entirely; when negative, falls back to
    /// `knob_arc_widths`. Allows the dimmed track to be thinner or thicker than
    /// the value arc.
    /// Default: matches `knob_arc_widths`.
    pub knob_arc_track_widths: [f32; 4],
    /// Glow intensity for the value arc, [0.0, 1.0]. 0.0 disables the glow.
    /// At 1.0, paints an outer halo at full configured opacity.
    /// Default: 0.0.
    pub knob_arc_glow: f32,
    /// Number of segments for arc rendering (smoothness).
    /// Default: 48.
    pub knob_arc_segments: u32,
    /// Knob border width in px.
    /// Default: 2.0.
    pub knob_border_width: f32,
    /// Layout style for the knob's title.
    /// 0=Boxed (chassis surrounds the knob), 1=Underlined (title above, thin
    /// underline rule, no surrounding chassis).
    /// Default: 0 (Boxed).
    pub knob_label_style: u8,
    /// Marker shape pointing at the current value.
    /// 0=Dot (filled circle), 1=Arrow (triangle pointing outward), 2=Tick
    /// (radial bar from indicator radius outward).
    /// Default: 0 (Dot).
    pub knob_indicator_style: u8,

    // -- VerticalSlider geometry --
    /// Slider track widths [Sm, Md, Lg] in px.
    /// Default: [14.0, 18.0, 24.0].
    pub slider_track_widths: [f32; 3],

    // -- Level meter geometry --
    /// Layout style for the level meter title.
    /// 0=Boxed (label sits inside a chassis box), 1=Underlined (label above,
    /// thin underline rule, bar below with no surrounding chassis).
    /// Default: 0 (Boxed).
    pub meter_label_style: u8,
    /// When true, the level meter fill is rendered with a luminance gradient
    /// (low values faded, high values luminous) instead of a flat color.
    /// Default: false.
    pub meter_use_gradient: bool,
    /// Corner radius (px) applied to the meter bar fill and track.
    /// 0.0 = square corners. The renderer clamps to half the bar's smaller axis.
    /// Default: 2.0 (matches the prior hardcoded value).
    pub meter_corner_radius: f32,
    /// Glow intensity for the meter / vertical-slider fill, [0.0, 1.0]. 0.0
    /// disables the halo. Higher values widen and brighten the colored
    /// box-shadow painted around the bar fill.
    /// Default: 0.0.
    pub meter_glow: f32,

    // -- Toggle variant --
    /// Toggle visual style.
    /// 0=Sliding (iOS capsule), 1=Segmented ([OFF|ON]), 2=ThumbOnTrack (Material), 3=Pill (Fluent).
    /// Default: 0 (Sliding).
    pub toggle_variant: u8,

    // -- Spacing --
    /// Control corner radius in px.
    /// Default: 8.0.
    pub corner_radius: f32,
    /// Minimum touch target size in px.
    /// Default: 32.0.
    pub min_touch_target: f32,
    /// Horizontal control padding in px.
    /// Default: 12.0.
    pub control_padding_x: f32,
    /// Vertical control padding in px.
    /// Default: 8.0.
    pub control_padding_y: f32,

    // -- Animation --
    /// Default animation duration in milliseconds.
    /// Default: 200.
    pub animation_duration_ms: u32,
    /// Whether to prefer spring physics over eased curves.
    /// Default: false.
    pub prefer_spring: bool,
    /// Spring stiffness (used when prefer_spring is true).
    /// Default: 170.0.
    pub spring_stiffness: f32,
    /// Spring damping (used when prefer_spring is true).
    /// Default: 26.0.
    pub spring_damping: f32,
}

impl Default for AudioDesignTokens {
    /// Returns tokens matching the current hardcoded values (neutral preset).
    fn default() -> Self {
        Self {
            knob_arc_start_deg: 135.0,
            knob_arc_sweep_deg: 270.0,
            knob_arc_widths: [2.5, 3.0, 3.5, 4.0],
            knob_arc_track_widths: [2.5, 3.0, 3.5, 4.0],
            knob_arc_glow: 0.0,
            knob_arc_segments: 48,
            knob_border_width: 2.0,
            knob_label_style: AudioDesignTokens::LABEL_BOXED,
            knob_indicator_style: AudioDesignTokens::INDICATOR_DOT,
            slider_track_widths: [14.0, 18.0, 24.0],
            meter_label_style: AudioDesignTokens::LABEL_BOXED,
            meter_use_gradient: false,
            meter_corner_radius: 2.0,
            meter_glow: 0.0,
            toggle_variant: 0,
            corner_radius: 8.0,
            min_touch_target: 32.0,
            control_padding_x: 12.0,
            control_padding_y: 8.0,
            animation_duration_ms: 200,
            prefer_spring: false,
            spring_stiffness: 170.0,
            spring_damping: 26.0,
        }
    }
}

impl From<&gpui_design::DesignSystem> for AudioDesignTokens {
    fn from(design: &gpui_design::DesignSystem) -> Self {
        let knob_arc_width = design.audio_controls.knob_arc_width;

        Self {
            knob_arc_start_deg: design.audio_controls.knob_arc_start_deg,
            knob_arc_sweep_deg: design.audio_controls.knob_arc_sweep_deg,
            knob_arc_widths: [
                knob_arc_width,
                knob_arc_width * 1.2,
                knob_arc_width * 1.4,
                knob_arc_width * 1.6,
            ],
            knob_arc_track_widths: [
                knob_arc_width,
                knob_arc_width * 1.2,
                knob_arc_width * 1.4,
                knob_arc_width * 1.6,
            ],
            knob_arc_glow: 0.0,
            knob_arc_segments: design.audio_controls.knob_arc_segments,
            knob_border_width: design.audio_controls.knob_border_width,
            knob_label_style: AudioDesignTokens::LABEL_BOXED,
            knob_indicator_style: AudioDesignTokens::INDICATOR_DOT,
            slider_track_widths: design.audio_controls.slider_track_widths,
            meter_label_style: AudioDesignTokens::LABEL_BOXED,
            meter_use_gradient: false,
            meter_corner_radius: design.corners.sm,
            meter_glow: 0.0,
            toggle_variant: match design.toggle_variant {
                gpui_design::ToggleVariant::Capsule => AudioDesignTokens::TOGGLE_SLIDING,
                gpui_design::ToggleVariant::Segmented => AudioDesignTokens::TOGGLE_SEGMENTED,
                gpui_design::ToggleVariant::ThumbOnTrack => {
                    AudioDesignTokens::TOGGLE_THUMB_ON_TRACK
                }
                gpui_design::ToggleVariant::Pill => AudioDesignTokens::TOGGLE_PILL,
            },
            corner_radius: design.corners.md,
            min_touch_target: design.interaction.min_touch_target,
            control_padding_x: design.spacing.control_padding_x,
            control_padding_y: design.spacing.control_padding_y,
            animation_duration_ms: design.animation.duration_ms,
            prefer_spring: design.animation.prefer_spring,
            spring_stiffness: design.animation.spring_stiffness,
            spring_damping: design.animation.spring_damping,
        }
    }
}

impl From<gpui_design::DesignSystem> for AudioDesignTokens {
    fn from(design: gpui_design::DesignSystem) -> Self {
        Self::from(&design)
    }
}

/// Toggle variant constants (matches design_system::ToggleVariant ordering).
impl AudioDesignTokens {
    pub const TOGGLE_SLIDING: u8 = 0;
    pub const TOGGLE_SEGMENTED: u8 = 1;
    pub const TOGGLE_THUMB_ON_TRACK: u8 = 2;
    pub const TOGGLE_PILL: u8 = 3;

    /// Label layout: enclose in a chassis box (current default).
    pub const LABEL_BOXED: u8 = 0;
    /// Label layout: title above with a thin underline rule, no chassis.
    pub const LABEL_UNDERLINED: u8 = 1;

    /// Indicator marker: filled dot.
    pub const INDICATOR_DOT: u8 = 0;
    /// Indicator marker: triangular arrow pointing outward.
    pub const INDICATOR_ARROW: u8 = 1;
    /// Indicator marker: radial tick line.
    pub const INDICATOR_TICK: u8 = 2;
}
