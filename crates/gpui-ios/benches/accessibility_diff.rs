use criterion::{Criterion, criterion_group, criterion_main};
use gpui_ios::accessibility::{
    AccessibilityDiffScratch, IosAccessibilityFrame, IosAccessibilityNode, IosAccessibilityRole,
    IosAccessibilitySnapshot, compute_accessibility_diff, compute_accessibility_diff_into,
};

fn frame_for_index(i: usize) -> IosAccessibilityFrame {
    IosAccessibilityFrame {
        x: (i % 10) as f32 * 44.0,
        y: (i / 10) as f32 * 44.0,
        width: 44.0,
        height: 44.0,
    }
}

fn snapshot_with_nodes(count: usize) -> IosAccessibilityNode {
    let mut root = IosAccessibilityNode::new("root", IosAccessibilityRole::Container);
    for i in 0..count {
        let id = format!("node-{i}");
        let node = IosAccessibilityNode::new(id, IosAccessibilityRole::Button)
            .label(format!("Button {i}"))
            .frame(frame_for_index(i));
        root = root.child(node);
    }
    root
}

fn apply_churn(root: &mut IosAccessibilityNode, churn_percent: usize) {
    if churn_percent == 0 {
        return;
    }
    let total = root.children.len();
    let churn_count = (total * churn_percent).div_ceil(100).max(1);
    for i in 0..churn_count {
        let idx = i % total;
        let child = &mut root.children[idx];
        child.label = Some(format!("Button {} updated", idx));
    }
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("accessibility_diff");

    for &size in &[100usize, 1_000, 5_000] {
        for &churn in &[0usize, 1, 5] {
            let prev_root = snapshot_with_nodes(size);
            let mut next_root = prev_root.clone();
            apply_churn(&mut next_root, churn);

            let prev = IosAccessibilitySnapshot::new(prev_root);
            let next = IosAccessibilitySnapshot::new(next_root);

            group.bench_function(format!("size_{size}_churn_{churn}_pct"), |b| {
                b.iter(|| compute_accessibility_diff(Some(&prev), &next));
            });
            let mut scratch = AccessibilityDiffScratch::default();
            compute_accessibility_diff_into(Some(&prev), &next, &mut scratch);
            group.bench_function(format!("size_{size}_churn_{churn}_pct_reuse"), |b| {
                b.iter(|| compute_accessibility_diff_into(Some(&prev), &next, &mut scratch));
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_diff);
criterion_main!(benches);
