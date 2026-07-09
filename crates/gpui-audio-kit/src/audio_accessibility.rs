use crate::accessibility::AriaRole;
use crate::scale::Scale;
use gpui::SharedString;

/// Inspectable accessibility metadata for audio-specific controls.
///
/// Components still register ARIA nodes during render. This summary gives tests,
/// hosts, and future native accessibility bridges a non-rendering contract for
/// control semantics, value ranges, and human-readable descriptions.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioAccessibilitySummary {
    pub control_type: &'static str,
    pub label: SharedString,
    pub role: AriaRole,
    pub value_now: Option<f64>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_text: Option<SharedString>,
    pub unit: Option<SharedString>,
    pub normalized: Option<f64>,
    pub scale: Option<Scale>,
    pub selected: bool,
    pub disabled: bool,
    pub muted: bool,
    pub peak_value: Option<f64>,
    pub description: SharedString,
}

pub(crate) fn normalized(value: f64, min: f64, max: f64, scale: Scale) -> f64 {
    scale.value_to_normalized(value, min, max)
}

pub(crate) fn value_text(value: f64, unit: &SharedString) -> SharedString {
    let unit = unit.as_ref();
    SharedString::new(if unit == ":1" {
        format!("{value:.1}{unit}")
    } else if unit.is_empty() {
        format!("{value:.1}")
    } else if unit == "%" {
        format!("{value:.0}{unit}")
    } else if unit == "Hz" {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.1} {unit}")
    })
}

pub(crate) fn range_description(
    control_type: &str,
    label: &SharedString,
    value_text: &SharedString,
    min: f64,
    max: f64,
    disabled: bool,
) -> SharedString {
    let disabled = if disabled { " Disabled." } else { "" };
    SharedString::new(format!(
        "{label}: {control_type} set to {value_text}, range {min:.1} to {max:.1}.{disabled}"
    ))
}
