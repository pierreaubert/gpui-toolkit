#![cfg(feature = "profiler-tests")]

use gpui::{TestAppContext, VisualTestContext};
use gpui_design::DesignSystemState;
use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_showcase::showcase::{Showcase, ShowcaseSection, allocation_contracts};
use gpui_ui_kit::{
    accessibility::AccessibilityTree,
    theme::{ThemeState, ThemeVariant},
};

#[gpui::test]
async fn warmed_showcase_interactions_stay_within_budget(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Light));
        app.set_global(DesignSystemState::new());
        app.set_global(AccessibilityTree::new());
    });

    let window = cx.add_window(|_window, entity_cx| Showcase::new(entity_cx));
    let state_window = window;
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();

    // Warm the selected section, keyboard path, and lazy QR entities first.
    state_window
        .update(&mut visual, |showcase, window, entity_cx| {
            allocation_contracts::select_section(
                showcase,
                ShowcaseSection::FormControls,
                entity_cx,
            );
            allocation_contracts::type_input_character(showcase, window, entity_cx);
            allocation_contracts::select_section(showcase, ShowcaseSection::QrCode, entity_cx);
        })
        .expect("warm showcase interactions");
    visual.run_until_parked();

    let mut switch_probe = AllocProbe::new();
    switch_probe.reset();
    state_window
        .update(&mut visual, |showcase, _window, entity_cx| {
            allocation_contracts::select_section(
                showcase,
                ShowcaseSection::FormControls,
                entity_cx,
            );
        })
        .expect("switch showcase section");
    visual.run_until_parked();

    let mut type_probe = AllocProbe::new();
    type_probe.reset();
    state_window
        .update(&mut visual, |showcase, window, entity_cx| {
            allocation_contracts::type_input_character(showcase, window, entity_cx);
        })
        .expect("type in showcase input");
    visual.run_until_parked();

    let mut qr_probe = AllocProbe::new();
    qr_probe.reset();
    state_window
        .update(&mut visual, |showcase, _window, entity_cx| {
            allocation_contracts::select_section(showcase, ShowcaseSection::QrCode, entity_cx);
        })
        .expect("show QR section");
    visual.run_until_parked();

    AllocationBudget::new("showcase-warmed-section-switch", 75_000, 8_000_000)
        .assert_contains(switch_probe.sample("showcase-warmed-section-switch"));
    AllocationBudget::new("showcase-warmed-input-typing", 68_000, 5_000_000)
        .assert_contains(type_probe.sample("showcase-warmed-input-typing"));
    AllocationBudget::new("showcase-warmed-qr-visible", 63_000, 2_000_000)
        .assert_contains(qr_probe.sample("showcase-warmed-qr-visible"));
}
