#![cfg(feature = "profiler")]

use gpui_profiler::AllocationBudget;
use gpui_showcase::showcase::warmed_navigation_input_sample;

#[test]
fn warmed_navigation_inputs_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }

    AllocationBudget::zero("showcase-warmed-navigation-inputs-1000x")
        .assert_contains(warmed_navigation_input_sample());
}
