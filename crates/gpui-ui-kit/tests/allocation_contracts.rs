//! Steady-state allocation contract for keyboard-driven text editing.

use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_ui_kit::input::edit_state::EditState;
use std::hint::black_box;

#[test]
fn warmed_edit_cycles_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }

    const ITERATIONS: usize = 1_000;
    let original = "word ".repeat(40);
    let mut state = EditState::new(&original);
    state.clear_selection();
    state.move_to_end();

    // Grow once so subsequent insertions reuse retained String capacity.
    state.insert_char('x');
    state.do_backspace();

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..ITERATIONS {
        state.insert_char('x');
        state.do_backspace();

        state.kill_word_backward();
        state.insert_text("word ");

        state.move_to_start();
        state.extend_forward();
        assert!(state.delete_selection());
        state.insert_char('w');
        state.move_to_end();
        black_box(&state);
    }

    AllocationBudget::zero("ui-kit-warmed-edit-cycle-1000x")
        .assert_contains(probe.sample("ui-kit-warmed-edit-cycle-1000x"));
}
