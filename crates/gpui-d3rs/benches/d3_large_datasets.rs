use std::cell::RefCell;
use std::hint::black_box;
use std::rc::Rc;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use d3rs::delaunay::Delaunay;
use d3rs::hexbin::Hexbin;
use d3rs::hierarchy::{HierarchyNode, TreeLayout};
use d3rs::quadtree::QuadTree;
use d3rs::sankey::{SankeyLayout, SankeyLinkInput};

fn deterministic_points(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|index| {
            let x = ((index * 37) % 10_007) as f64 + (index % 13) as f64 * 0.01;
            let y = ((index * 91) % 10_009) as f64 + (index % 17) as f64 * 0.01;
            (x, y)
        })
        .collect()
}

fn deterministic_queries(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|index| {
            let x = ((index * 211) % 10_007) as f64 + 0.5;
            let y = ((index * 307) % 10_009) as f64 + 0.25;
            (x, y)
        })
        .collect()
}

fn sankey_fixture() -> (Vec<String>, Vec<SankeyLinkInput>) {
    const LAYERS: usize = 12;
    const NODES_PER_LAYER: usize = 20;

    let node_names: Vec<_> = (0..LAYERS)
        .flat_map(|layer| (0..NODES_PER_LAYER).map(move |slot| format!("n{layer}_{slot}")))
        .collect();

    let mut links = Vec::with_capacity(720);
    for layer in 0..LAYERS - 1 {
        for slot in 0..NODES_PER_LAYER {
            for offset in 0..3 {
                links.push(sankey_link(layer, slot, offset, 1.0 + offset as f64));
            }
            if layer < 3 {
                links.push(sankey_link(layer, slot, 7, 0.5));
            }
        }
    }

    (node_names, links)
}

fn sankey_link(layer: usize, slot: usize, offset: usize, value: f64) -> SankeyLinkInput {
    let target_slot = (slot + offset) % 20;
    SankeyLinkInput {
        source: format!("n{layer}_{slot}"),
        target: format!("n{}_{target_slot}", layer + 1),
        value,
    }
}

fn balanced_hierarchy(max_depth: usize) -> Rc<RefCell<HierarchyNode<usize>>> {
    fn build(
        depth: usize,
        max_depth: usize,
        next_id: &mut usize,
    ) -> Rc<RefCell<HierarchyNode<usize>>> {
        let node = HierarchyNode::new(*next_id);
        *next_id += 1;

        if depth < max_depth {
            let children = vec![
                build(depth + 1, max_depth, next_id),
                build(depth + 1, max_depth, next_id),
            ];
            node.borrow_mut().set_children(&node, children);
        }

        node
    }

    let mut next_id = 0;
    build(0, max_depth, &mut next_id)
}

fn quadtree_benchmarks(c: &mut Criterion) {
    let points = deterministic_points(10_000);
    let queries = deterministic_queries(512);
    let tree = QuadTree::try_from_data(&points, |point| point.0, |point| point.1).unwrap();

    let mut group = c.benchmark_group("quadtree");
    group.bench_function("try_from_data/10000", |b| {
        b.iter(|| {
            QuadTree::try_from_data(black_box(&points), |point| point.0, |point| point.1).unwrap()
        });
    });
    group.bench_function("find/10000", |b| {
        b.iter(|| {
            let mut hits = 0usize;
            for &(x, y) in &queries {
                if tree.find(black_box(x), black_box(y), None).is_some() {
                    hits += 1;
                }
            }
            black_box(hits)
        });
    });
    group.finish();
}

fn hexbin_benchmarks(c: &mut Criterion) {
    let points = deterministic_points(20_000);
    let hexbin = Hexbin::with_accessors(|point: &(f64, f64)| point.0, |point| point.1)
        .radius(20.0)
        .extent(0.0, 0.0, 10_100.0, 10_100.0);

    let mut group = c.benchmark_group("hexbin");
    group.bench_function("try_bin/20000", |b| {
        b.iter_batched(
            || points.clone(),
            |points| hexbin.try_bin(black_box(points)).unwrap(),
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn delaunay_benchmarks(c: &mut Criterion) {
    let points = deterministic_points(2_500);
    let delaunay = Delaunay::try_new(&points).unwrap();
    let bounds = [0.0, 0.0, 10_100.0, 10_100.0];
    let mut path = String::new();

    let mut group = c.benchmark_group("delaunay");
    group.bench_function("try_new/2500", |b| {
        b.iter(|| Delaunay::try_new(black_box(&points)).unwrap());
    });
    group.bench_function("voronoi_render/2500", |b| {
        b.iter(|| {
            path.clear();
            let voronoi = delaunay.try_voronoi(Some(bounds)).unwrap();
            voronoi.render_to_path_into(&mut path);
            black_box(path.len())
        });
    });
    group.finish();
}

fn sankey_benchmarks(c: &mut Criterion) {
    let (node_names, links) = sankey_fixture();
    let layout = SankeyLayout::new()
        .width(1_200.0)
        .height(900.0)
        .node_padding(2.0)
        .iterations(8);

    let mut group = c.benchmark_group("sankey");
    group.bench_function("try_compute/240x720", |b| {
        b.iter(|| {
            layout
                .try_compute(black_box(&node_names), black_box(&links))
                .unwrap()
        });
    });
    group.finish();
}

fn hierarchy_benchmarks(c: &mut Criterion) {
    let layout = TreeLayout::new().size((1_024.0, 1_024.0));

    let mut group = c.benchmark_group("hierarchy");
    group.bench_function("tree_try_layout/4095", |b| {
        b.iter_batched(
            || balanced_hierarchy(11),
            |root| layout.try_layout(black_box(root)).unwrap(),
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    quadtree_benchmarks,
    hexbin_benchmarks,
    delaunay_benchmarks,
    sankey_benchmarks,
    hierarchy_benchmarks,
);
criterion_main!(benches);
