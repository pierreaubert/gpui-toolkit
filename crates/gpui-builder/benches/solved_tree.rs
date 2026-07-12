use criterion::{Criterion, criterion_group, criterion_main};
use gpui_builder::{
    Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode, SolvedNode, SolvedTree,
    solve, solve_tree, solve_tree_into,
};
use std::hint::black_box;

fn make_balanced_tree(depth: usize, counter: &mut usize) -> LayoutNode<'static> {
    if depth == 0 {
        let id = format!("leaf-{}", *counter);
        *counter += 1;
        // Leak the string so it has a 'static lifetime.
        let id: &'static str = Box::leak(id.into_boxed_str());
        LayoutNode::Slot(SlotNode {
            id,
            sizing: Sizing::flex(1.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })
    } else {
        let left = make_balanced_tree(depth - 1, counter);
        let right = make_balanced_tree(depth - 1, counter);
        let children = Box::leak(vec![left, right].into_boxed_slice());
        let id = format!("node-{}", *counter);
        *counter += 1;
        let id: &'static str = Box::leak(id.into_boxed_str());
        LayoutNode::Container(ContainerNode {
            id,
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children,
            divider_size: 0.0,
        })
    }
}

fn make_wide_tree(count: usize) -> LayoutNode<'static> {
    let mut children = Vec::with_capacity(count);
    for i in 0..count {
        let id: &'static str = Box::leak(format!("leaf-{i}").into_boxed_str());
        children.push(LayoutNode::Slot(SlotNode {
            id,
            sizing: Sizing::flex(1.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }));
    }
    let children = Box::leak(children.into_boxed_slice());
    LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children,
        divider_size: 0.0,
    })
}

/// A deterministic, cheap text measurer for benchmarking the text-measurement
/// cache-hit path without the noise of a real shaping engine.
struct FixedWidthMeasure {
    char_width: f64,
}

impl gpui_pretext::TextMeasure for FixedWidthMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * self.char_width
    }
}

/// Build a layout tree containing `count` `Sizing::Text` children.
fn make_text_tree(count: usize) -> LayoutNode<'static> {
    let measure: &'static FixedWidthMeasure =
        Box::leak(Box::new(FixedWidthMeasure { char_width: 8.0 }));

    let mut children = Vec::with_capacity(count);
    for i in 0..count {
        let id: &'static str = Box::leak(format!("text-{i}").into_boxed_str());
        let text: &'static str = Box::leak(format!("Text slot {i}").into_boxed_str());
        children.push(LayoutNode::Slot(SlotNode {
            id,
            sizing: Sizing::Text {
                text,
                measure,
                line_height: 20.0,
                min: 0.0,
            },
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }));
    }

    let children = Box::leak(children.into_boxed_slice());
    LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children,
        divider_size: 0.0,
    })
}

fn benchmark_find(c: &mut Criterion) {
    let prefs = LayoutPreferences::default();

    let mut group = c.benchmark_group("balanced_tree_find");
    for depth in [6, 8, 10] {
        let mut counter = 0;
        let root = make_balanced_tree(depth, &mut counter);
        let recursive = solve(&root, 1000.0, 1000.0, &prefs);
        let flat = solve_tree(&root, 1000.0, 1000.0, &prefs);
        let recursive_tree: SolvedTree = recursive.clone().into_tree();
        let target = format!("leaf-{}", counter - 1);

        group.bench_function(format!("recursive_find_depth_{depth}"), |b| {
            b.iter(|| black_box(recursive.find(black_box(&target))));
        });
        group.bench_function(format!("as_map_get_depth_{depth}"), |b| {
            let map = recursive_tree.as_map();
            b.iter(|| black_box(map.get(black_box(target.as_str()))));
        });
        group.bench_function(format!("flat_find_depth_{depth}"), |b| {
            b.iter(|| black_box(flat.find(black_box(&target))));
        });
    }
    group.finish();

    let mut group = c.benchmark_group("wide_tree_find");
    for count in [50, 200, 500] {
        let root = make_wide_tree(count);
        let recursive = solve(&root, 1000.0, 1000.0, &prefs);
        let flat = solve_tree(&root, 1000.0, 1000.0, &prefs);
        let recursive_tree: SolvedTree = recursive.clone().into_tree();
        let target = format!("leaf-{}", count / 2);

        group.bench_function(format!("recursive_find_count_{count}"), |b| {
            b.iter(|| black_box(recursive.find(black_box(&target))));
        });
        group.bench_function(format!("as_map_get_count_{count}"), |b| {
            let map = recursive_tree.as_map();
            b.iter(|| black_box(map.get(black_box(target.as_str()))));
        });
        group.bench_function(format!("flat_find_count_{count}"), |b| {
            b.iter(|| black_box(flat.find(black_box(&target))));
        });
    }
    group.finish();
}

fn benchmark_traversal(c: &mut Criterion) {
    let prefs = LayoutPreferences::default();

    let mut group = c.benchmark_group("balanced_tree_traversal");
    for depth in [6, 8, 10] {
        let mut counter = 0;
        let root = make_balanced_tree(depth, &mut counter);
        let recursive = solve(&root, 1000.0, 1000.0, &prefs);
        let flat = solve_tree(&root, 1000.0, 1000.0, &prefs);

        group.bench_function(format!("recursive_collect_depth_{depth}"), |b| {
            b.iter(|| {
                let mut ids = Vec::new();
                collect_recursive_ids(black_box(&recursive), &mut ids);
                black_box(ids);
            });
        });
        group.bench_function(format!("flat_iter_depth_{depth}"), |b| {
            b.iter(|| {
                let ids: Vec<&str> = flat.iter().map(|n| n.id()).collect();
                black_box(ids);
            });
        });
    }
    group.finish();
}

fn benchmark_text_cache_hit(c: &mut Criterion) {
    let prefs = LayoutPreferences::default();
    let root = make_text_tree(20);

    // Warm the thread-local text-measurement cache so the benchmarked calls
    // hit the cache instead of running real text layout.
    let _warm = solve(&root, 400.0, 2000.0, &prefs);

    let mut group = c.benchmark_group("text_cache_hit");
    group.bench_function("solve_text_cache_hit", |b| {
        b.iter(|| {
            let solved = solve(
                black_box(&root),
                black_box(400.0),
                black_box(2000.0),
                &prefs,
            );
            black_box(solved);
        });
    });
    group.bench_function("solve_tree_text_cache_hit", |b| {
        b.iter(|| {
            let solved = solve_tree(
                black_box(&root),
                black_box(400.0),
                black_box(2000.0),
                &prefs,
            );
            black_box(solved);
        });
    });
    let mut reusable = SolvedTree::with_capacity(root.node_count());
    solve_tree_into(&root, 400.0, 2000.0, &prefs, &mut reusable);
    group.bench_function("solve_tree_into_text_cache_hit", |b| {
        b.iter(|| {
            solve_tree_into(
                black_box(&root),
                black_box(400.0),
                black_box(2000.0),
                &prefs,
                &mut reusable,
            );
            black_box(&reusable);
        });
    });
    group.finish();
}

fn collect_recursive_ids<'a>(node: &'a SolvedNode<'a>, out: &mut Vec<&'a str>) {
    out.push(node.id);
    for child in &node.children {
        collect_recursive_ids(child, out);
    }
}

criterion_group!(
    benches,
    benchmark_find,
    benchmark_traversal,
    benchmark_text_cache_hit
);
criterion_main!(benches);
