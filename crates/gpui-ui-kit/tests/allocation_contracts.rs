//! Steady-state allocation contract for keyboard-driven text editing.

use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_ui_kit::input::edit_state::EditState;
use gpui_ui_kit::workflow::{NodeDragState, Position};
use std::collections::HashMap;
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

    let node_id = uuid::Uuid::nil();
    let mut original_positions = HashMap::new();
    original_positions.insert(node_id, Position::new(10.0, 20.0));
    let mut drag = NodeDragState {
        dragging_nodes: vec![node_id],
        start_mouse: Position::new(0.0, 0.0),
        original_positions,
    };
    probe.reset();
    for frame in 0..ITERATIONS {
        drag.start_mouse.x = frame as f32 * 0.1;
        drag.start_mouse.y = frame as f32 * 0.05;
        black_box(drag.original_positions.get(&node_id));
    }
    AllocationBudget::zero("workflow-node-drag-update-1000x")
        .assert_contains(probe.sample("workflow-node-drag-update-1000x"));
}
