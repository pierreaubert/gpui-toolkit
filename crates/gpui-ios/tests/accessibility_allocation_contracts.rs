//! Allocation contract for host-runnable iOS accessibility snapshot diffs.

use gpui_ios::accessibility::{
    AccessibilityDiffScratch, IosAccessibilityNode, IosAccessibilityRole, IosAccessibilitySnapshot,
    compute_accessibility_diff_into,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

fn snapshot(count: usize, changed: usize) -> IosAccessibilitySnapshot {
    let mut root = IosAccessibilityNode::new("root", IosAccessibilityRole::Container);
    for index in 0..count {
        let label = if index < changed {
            format!("Button {index} updated")
        } else {
            format!("Button {index}")
        };
        root.children.push(
            IosAccessibilityNode::new(format!("node-{index}"), IosAccessibilityRole::Button)
                .label(label),
        );
    }
    IosAccessibilitySnapshot::new(root)
}

#[test]
fn warmed_accessibility_snapshot_diffs_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }

    let previous = snapshot(1_000, 0);
    let next = snapshot(1_000, 10);
    let mut scratch = AccessibilityDiffScratch::default();

    compute_accessibility_diff_into(Some(&previous), &next, &mut scratch);
    assert_eq!(scratch.changed_indices().len(), 10);
    assert_eq!(scratch.unchanged_indices().len(), 990);

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        compute_accessibility_diff_into(Some(&previous), &next, &mut scratch);
        black_box(&scratch);
    }

    AllocationBudget::zero("ios-accessibility-diff-1000-nodes-1000x")
        .assert_contains(probe.sample("ios-accessibility-diff-1000-nodes-1000x"));
}
