//! Stable allocation checks that run in their own integration-test process.
//!
//! `AllocProbe` observes the process-wide allocator, so putting this workload
//! in the unit-test binary makes parallel tests appear as false positives.

use super::misc::{
    boxplot_story_data, scalar_field_data, scatter_story_data, spectrum_axis_magnitudes,
    spectrum_magnitudes,
};
use super::types::{area_story_data, bar_story_data, line_story_data};
use super::{ComponentLab, LabAppConfig};
use crate::StoryPropValue;
use gpui::Context;
use gpui_profiler::{AllocProbe, AllocSnapshot};
use std::hint::black_box;

/// Measures a warmed-up chart-data workload without unrelated test activity.
pub fn warmed_chart_story_data_sample() -> AllocSnapshot {
    black_box(line_story_data("sine"));
    black_box(area_story_data("envelope"));
    black_box(bar_story_data(12));
    black_box(scatter_story_data(24));
    black_box(boxplot_story_data(4));
    black_box(scalar_field_data(8, 8));
    black_box(spectrum_magnitudes(32));
    black_box(spectrum_axis_magnitudes());

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        black_box(line_story_data("sine"));
        black_box(area_story_data("envelope"));
        black_box(bar_story_data(12));
        black_box(scatter_story_data(24));
        black_box(boxplot_story_data(4));
        black_box(scalar_field_data(8, 8));
        black_box(spectrum_magnitudes(32));
        black_box(spectrum_axis_magnitudes());
    }
    probe.sample("component-lab-warmed-chart-data-1000x")
}

/// Constructs a lab instance for the isolated visual allocation contract.
pub fn new_component_lab(config: LabAppConfig, cx: &mut Context<ComponentLab>) -> ComponentLab {
    ComponentLab::new(config, cx)
}

/// Applies one valid selected-story prop mutation and returns its allocation sample.
pub fn selected_story_prop_change_sample(
    lab: &mut ComponentLab,
    cx: &mut Context<ComponentLab>,
) -> AllocSnapshot {
    let story_id = lab.selected_story_id.clone();
    let (prop_name, value): (String, StoryPropValue) = lab.documents[&story_id]
        .story
        .props
        .first()
        .map(|prop| (prop.name.clone(), changed_prop_value(&prop.value)))
        .expect("every selected component-lab story has a prop for the allocation contract");
    lab.set_prop_without_notify(&story_id, &prop_name, value);
    let sample = lab
        .last_allocation_sample()
        .expect("set_prop records a profiler allocation sample")
        .1;
    cx.notify();
    sample
}

/// Clears the process-wide allocation interval after visual work settles so
/// the next interaction sample excludes the preceding render.
pub fn reset_after_render(lab: &mut ComponentLab) {
    lab.reset_allocation_delta();
}

fn changed_prop_value(value: &StoryPropValue) -> StoryPropValue {
    match value {
        StoryPropValue::Bool(value) => StoryPropValue::Bool(!value),
        StoryPropValue::Number(value) => StoryPropValue::Number(value + 1.0),
        StoryPropValue::Text(value) => StoryPropValue::Text(format!("{value} profile").into()),
        // Choice and color values are constrained by a story's declared options.
        // Keeping their valid value still exercises the control's update path.
        StoryPropValue::Choice(value) => StoryPropValue::Choice(value.clone()),
        StoryPropValue::Color(value) => StoryPropValue::Color(value.clone()),
    }
}

/// Returns the render allocation sample recorded by the retained lab entity.
pub fn last_render_sample(lab: &ComponentLab) -> AllocSnapshot {
    lab.last_render_allocation_sample()
}
