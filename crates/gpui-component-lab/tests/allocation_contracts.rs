#![cfg(feature = "profiler")]

use gpui_component_lab::lab_ui::allocation_contracts::warmed_chart_story_data_sample;
use gpui_profiler::AllocationBudget;

#[test]
fn warmed_chart_story_data_is_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }

    AllocationBudget::zero("component-lab-warmed-chart-data-1000x")
        .assert_contains(warmed_chart_story_data_sample());
}
