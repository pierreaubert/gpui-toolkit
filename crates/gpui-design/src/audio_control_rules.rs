use serde::{Deserialize, Serialize};

/// Audio control geometry — knob arc, slider tracks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioControlRules {
    /// Knob arc start angle in degrees from 12 o'clock, clockwise.
    pub knob_arc_start_deg: f32,
    /// Knob arc sweep in degrees (dead zone at bottom = 360 - sweep).
    pub knob_arc_sweep_deg: f32,
    /// Arc thickness in px.
    pub knob_arc_width: f32,
    /// Number of segments for arc rendering (smoothness).
    pub knob_arc_segments: u32,
    /// Knob border width in px.
    pub knob_border_width: f32,
    /// Slider track widths [Sm, Md, Lg] in px.
    pub slider_track_widths: [f32; 3],
}

impl AudioControlRules {
    pub fn new(
        knob_arc_start_deg: f32,
        knob_arc_sweep_deg: f32,
        knob_arc_width: f32,
        knob_arc_segments: u32,
        knob_border_width: f32,
        slider_track_widths: [f32; 3],
    ) -> Self {
        assert!(
            knob_arc_start_deg.is_finite(),
            "knob_arc_start_deg must be finite"
        );
        assert!(
            knob_arc_sweep_deg.is_finite()
                && knob_arc_sweep_deg > 0.0
                && knob_arc_sweep_deg <= 360.0,
            "knob_arc_sweep_deg must be finite and in (0, 360]"
        );
        assert!(
            knob_arc_segments >= 12,
            "knob_arc_segments must be at least 12"
        );
        assert!(
            knob_arc_width.is_finite() && knob_arc_width >= 0.0,
            "knob_arc_width must be finite and >= 0"
        );
        assert!(
            knob_border_width.is_finite() && knob_border_width >= 0.0,
            "knob_border_width must be finite and >= 0"
        );
        for (i, &w) in slider_track_widths.iter().enumerate() {
            assert!(
                w.is_finite() && w > 0.0,
                "slider_track_widths[{i}] must be finite and > 0"
            );
        }
        Self {
            knob_arc_start_deg,
            knob_arc_sweep_deg,
            knob_arc_width,
            knob_arc_segments,
            knob_border_width,
            slider_track_widths,
        }
    }
}
