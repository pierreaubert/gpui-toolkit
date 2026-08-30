//! Steady-state allocation contract for frame-rate responsive solves.

use gpui_builder::{
    Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode, SolvedTree,
    solve_tree_into,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn warmed_reusable_layout_solves_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    let nested_children = [
        LayoutNode::Slot(
            SlotNode::new("left", Sizing::fractional(0.3, 80.0)).collapsible(0.1, "Left"),
        ),
        LayoutNode::slot("main", Sizing::flex(120.0)),
    ];
    let root_children = [
        LayoutNode::slot("header", Sizing::Fixed(48.0)),
        ContainerNode::new(
            "content",
            Axis::Horizontal,
            Sizing::flex(0.0),
            &nested_children,
        )
        .divider_size(6.0)
        .into_node(),
        LayoutNode::Slot(SlotNode::new("footer", Sizing::Fixed(32.0)).collapsible(0.2, "Footer")),
    ];
    let root =
        ContainerNode::new("root", Axis::Vertical, Sizing::flex(0.0), &root_children).into_node();
    let preferences = LayoutPreferences::default();
    let mut target = SolvedTree::with_capacity(root.node_count());

    // Exercise every structural layout state first. Narrow widths can collapse
    // a nested container, while wider widths restore it and its recycled
    // buffers. Only measure the steady-state resize pass after both shapes
    // have populated the retained pools.
    solve_tree_into(&root, 1_200.0, 800.0, &preferences, &mut target);
    for width in 0..1_100 {
        solve_tree_into(
            &root,
            100.0 + width as f32,
            800.0,
            &preferences,
            &mut target,
        );
    }

    let mut probe = AllocProbe::new();
    probe.reset();
    for width in 0..1_000 {
        solve_tree_into(
            &root,
            100.0 + (width % 1_100) as f32,
            800.0,
            &preferences,
            &mut target,
        );
        black_box(target.find("main").unwrap().width());
        for slot in target.collapsed_slots() {
            black_box(slot.id);
            black_box(slot.label);
        }
    }

    AllocationBudget::zero("builder-reusable-layout-resize-1000x")
        .assert_contains(probe.sample("builder-reusable-layout-resize-1000x"));
}
