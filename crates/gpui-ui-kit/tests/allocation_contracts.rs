//! Steady-state allocation contract for keyboard-driven text editing.

use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_ui_kit::input::edit_state::EditState;
use gpui_ui_kit::workflow::{NodeDragState, Position};
use std::collections::HashMap;
use std::hint::black_box;

fn edit_cycle(state: &mut EditState) {
    state.insert_char('x');
    state.do_backspace();
    state.kill_word_backward();
    state.insert_text("word ");
    state.move_to_start();
    state.extend_forward();
    assert!(state.delete_selection());
    state.insert_char('w');
    state.move_to_end();
}

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

    // Fill and rotate bounded undo history through every edit phase before
    // measuring. Each mutation has a different text length, so a single fill
    // can leave a recycled slot too small on a different allocator.
    for _ in 0..1_000 {
        edit_cycle(&mut state);
    }

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..ITERATIONS {
        edit_cycle(&mut state);
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

#[test]
fn table_pure_ops_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    use gpui_ui_kit::data_navigation::DataVirtualWindow;
    use gpui_ui_kit::table::PaginationState;
    use std::collections::HashSet;

    const ITERATIONS: usize = 1_000;
    let mut probe = AllocProbe::new();

    // Pagination math: pure arithmetic over the current page.
    probe.reset();
    let mut page = 0;
    for _ in 0..ITERATIONS {
        page = (page + 1) % 200;
        let state = PaginationState {
            current_page: page,
            page_size: 50,
            total_items: 10_000,
        };
        black_box((state.total_pages(), state.page_range()));
    }
    AllocationBudget::zero("table-pagination-math-1000x")
        .assert_contains(probe.sample("table-pagination-math-1000x"));

    // Virtual-window scroll math: pure float arithmetic, no allocation.
    probe.reset();
    let mut offset = 0.0f32;
    for _ in 0..ITERATIONS {
        offset += 32.0;
        if offset > 320_000.0 {
            offset = 0.0;
        }
        black_box(DataVirtualWindow::from_viewport(
            10_000, offset, 32.0, 960.0, 5,
        ));
    }
    AllocationBudget::zero("table-virtual-window-math-1000x")
        .assert_contains(probe.sample("table-virtual-window-math-1000x"));

    // Selection toggle on a pre-sized set: steady state must not regrow.
    let mut selected: HashSet<usize> = HashSet::with_capacity(64);
    for i in 0..64 {
        selected.insert(i);
    }
    probe.reset();
    for i in 0..ITERATIONS {
        let index = i % 1_000;
        if !selected.remove(&index) {
            selected.insert(index);
        }
        black_box(&selected);
    }
    // Note: inserts past capacity regrow; the toggle range exceeds the
    // pre-size, so this asserts a bounded regrowth budget instead of zero.
    let sample = probe.sample("table-selection-toggle-1000x");
    AllocationBudget::new("table-selection-toggle-1000x", 32, 65_536).assert_contains(sample);
}

/// Per-frame allocation budget for a 100-row table redraw (warmed).
///
/// Element trees rebuild every frame by design, so this cannot be zero;
/// the budget ratchets the steady-state cost instead: any per-cell
/// allocation added later (a `format!`, an unhoisted `to_string`) shows up
/// here before it shows up in frame time.
#[test]
fn table_redraw_100_stays_within_budget() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    use gpui::{
        AnyWindowHandle, AppContext as _, Context, HeadlessAppContext, IntoElement,
        PlatformTextSystem as _, Render, SharedString, Window, WindowHandle, px, size,
    };
    use gpui_design::{DesignSystem, DesignSystemState};
    use gpui_ui_kit::accessibility::AccessibilityTree;
    use gpui_ui_kit::i18n::I18nState;
    use gpui_ui_kit::table::{Column, SelectionMode, Table};
    use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[derive(Clone)]
    struct Row {
        id_text: SharedString,
        name: SharedString,
    }

    struct TableView {
        rows: Vec<Row>,
    }

    impl Render for TableView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            Table::new("contract-table", self.rows.clone())
                .columns(vec![
                    Column::new("id", "ID")
                        .width(px(60.0))
                        .cell_render(|row: &Row, _, _, _| row.id_text.clone()),
                    Column::new("name", "Name").cell_render(|row: &Row, _, _, _| row.name.clone()),
                ])
                .selection_mode(SelectionMode::Multiple)
                .selected_indices(HashSet::new())
        }
    }

    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new_without_system_fonts(
        "IBM Plex Sans",
    ));
    text_system
        .add_fonts(gpui_miniapp::bundled_ui_fonts())
        .unwrap();
    let text_system: Arc<dyn gpui::PlatformTextSystem> = text_system;
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || Ok(None));
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Dark));
        app.set_global(DesignSystemState::with_system(DesignSystem::neutral()));
        app.set_global(AccessibilityTree::new());
        app.set_global(I18nState::new());
    });

    let rows: Vec<Row> = (0..100)
        .map(|id| Row {
            id_text: id.to_string().into(),
            name: format!("User {id:05}").into(),
        })
        .collect();
    let window: WindowHandle<TableView> = cx
        .open_window(size(px(1200.0), px(900.0)), |_, app| {
            app.new(|_| TableView { rows })
        })
        .unwrap();
    let any: AnyWindowHandle = *window;
    let draw = |cx: &mut HeadlessAppContext| {
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.update_window(any, |_view, window, app| {
            let _ = window.draw(app);
        })
        .unwrap();
    };

    // Warm shaping caches, text system internals, and allocator freelists.
    for _ in 0..10 {
        draw(&mut cx);
    }

    let mut probe = AllocProbe::new();
    probe.reset();
    draw(&mut cx);
    let sample = probe.sample("table-redraw-100-1x");
    if std::env::var_os("ALLOC_CONTRACTS_PRINT").is_some() {
        eprintln!(
            "table-redraw-100-1x: {} calls/{} bytes ({} reallocs)",
            sample.count, sample.bytes, sample.reallocs
        );
    }
    // Calibrated 2026-10-10 (debug): 19,260 calls / 9.09 MB warmed. Ratchet
    // down as per-frame waste is removed; the count budget is the binding
    // one (per-cell/per-row allocations), bytes have allocator headroom.
    AllocationBudget::new("table-redraw-100-1x", 21_000, 12_000_000).assert_contains(sample);
}
