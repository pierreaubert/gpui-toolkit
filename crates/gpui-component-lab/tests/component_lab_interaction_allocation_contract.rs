#![cfg(all(feature = "profiler", feature = "visual-capture"))]

use gpui::{TestAppContext, VisualTestContext};
use gpui_component_lab::lab_ui::{LabAppConfig, allocation_contracts};
use gpui_design::DesignSystemState;
use gpui_profiler::AllocationBudget;
use gpui_ui_kit::{
    accessibility::AccessibilityTree,
    theme::{ThemeState, ThemeVariant},
};

#[gpui::test]
async fn warmed_component_lab_render_and_prop_change_stay_within_budget(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Light));
        app.set_global(DesignSystemState::new());
        app.set_global(AccessibilityTree::new());
    });

    let stories_dir = tempfile::tempdir().expect("temporary story directory");
    let config = LabAppConfig::new(stories_dir.path().to_path_buf(), Vec::new());
    let window = cx.add_window(|_window, entity_cx| {
        allocation_contracts::new_component_lab(config, entity_cx)
    });
    let state_window = window;
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    state_window
        .update(&mut visual, |lab, _window, _cx| {
            allocation_contracts::reset_after_render(lab);
        })
        .expect("reset initial component-lab allocation interval");

    // Warm the retained child entities and initial render before measuring the
    // prop mutation and its following redraw.
    state_window
        .update(&mut visual, |lab, _window, cx| {
            allocation_contracts::selected_story_prop_change_sample(lab, cx);
        })
        .expect("update component lab");
    visual.run_until_parked();
    state_window
        .update(&mut visual, |lab, _window, _cx| {
            allocation_contracts::reset_after_render(lab);
        })
        .expect("reset warmed component-lab allocation interval");

    let prop_change = state_window
        .update(&mut visual, |lab, _window, cx| {
            allocation_contracts::selected_story_prop_change_sample(lab, cx)
        })
        .expect("sample component-lab prop change");
    visual.run_until_parked();
    let render = state_window
        .update(&mut visual, |lab, _window, _cx| {
            allocation_contracts::last_render_sample(lab)
        })
        .expect("sample component-lab render");

    AllocationBudget::new("component-lab-warmed-prop-change", 6, 128).assert_contains(prop_change);
    AllocationBudget::new("component-lab-warmed-render", 175, 16_000).assert_contains(render);
}
