use super::row_spec::RowSpec;
use crate::compat::KnobSize;

/// Header strip at the top of a plugin chassis.
#[derive(Debug, Clone)]
pub struct HeaderSpec {
    /// Small caps mark in the top-left (e.g. "SOTF · Module 07").
    pub brand_mark: String,
    /// Italic display title in the center (e.g. "Loudness").
    pub title: String,
    /// Caption below the title (e.g. "Equal-Loudness Compensation · ISO 226:2003").
    pub subtitle: String,
}

/// Footer strip at the bottom of a plugin chassis.
#[derive(Debug, Clone)]
pub struct FooterSpec {
    /// Tick segments on the left (sample rate, latency, CPU, etc.).
    pub ticks: Vec<String>,
    /// Build / serial readout on the right.
    pub serial: String,
}

/// One numbered section of the chassis (e.g. "01 · Reference").
#[derive(Debug, Clone)]
pub struct SectionSpec {
    /// Stable id used by the renderer to look up the section's layout slot.
    pub id: String,
    /// Eyebrow label shown before the title (e.g. "01").
    pub eyebrow: String,
    /// Italic display title (e.g. "Reference", "Low", "Output").
    pub title: String,
    /// Optional caption right-aligned in the section header (e.g. "SPL", "band 1").
    pub caption: Option<String>,
    /// Vertical stack of rows inside the section.
    pub rows: Vec<RowSpec>,
    /// Minimum width at which the section is rendered. Below this, the
    /// section collapses to a tab.
    pub min_width: f32,
    /// Preferred width when there is enough space; the section grows up to
    /// this value before flexing.
    pub preferred_width: f32,
    /// Collapse priority — lowest values collapse first when space is tight.
    /// 1.0 = never collapses. The Main / wide section should be 1.0.
    pub priority: f32,
}

/// A single knob slot inside a [`RowSpec::KnobRow`].
#[derive(Debug, Clone)]
pub struct KnobSlot {
    /// Stable id (e.g. "freq", "gain").
    pub id: String,
    /// Index into the plugin's parameter list — the renderer uses this to
    /// look up the live value and bind drag handlers.
    pub param_idx: usize,
    /// Visible label below the knob (e.g. "Frequency").
    pub label: String,
    /// Knob size tier — `KnobSize` is reused from `gpui_builder::compat`.
    pub size: KnobSize,
    /// `true` for ± gain knobs that draw outward from 12 o'clock.
    pub bipolar: bool,
}

#[derive(Debug, Clone)]
pub struct SolvedSection {
    pub id: String,
    pub width: f32,
    pub visible: bool,
}
