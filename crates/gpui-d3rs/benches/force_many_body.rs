use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use d3rs::force::{Force, ForceManyBody, SimulationNode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn deterministic_nodes(n: usize) -> Vec<Rc<RefCell<SimulationNode>>> {
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let x = (i as f64 * 0.618033988749895).fract() * 100.0;
        let y = (i as f64 * 0.381966011250105).fract() * 100.0;
        nodes.push(SimulationNode::new(i, x, y));
    }
    nodes
}

fn clone_nodes(nodes: &[Rc<RefCell<SimulationNode>>]) -> Vec<Rc<RefCell<SimulationNode>>> {
    nodes
        .iter()
        .map(|n| {
            let n = n.borrow();
            SimulationNode::new(n.index, n.x, n.y)
        })
        .collect()
}

fn bench_many_body(c: &mut Criterion) {
    let mut group = c.benchmark_group("force_many_body");
    group.measurement_time(Duration::from_secs(1));
    group.sample_size(20);

    for n in [50, 100, 500, 1_000, 5_000] {
        let nodes = deterministic_nodes(n);

        group.bench_with_input(BenchmarkId::new("brute_force", n), &n, |b, _| {
            b.iter_batched(
                || clone_nodes(&nodes),
                |clone| {
                    let mut force = ForceManyBody::new();
                    force.force(black_box(1.0), &clone);
                },
                criterion::BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("barnes_hut", n), &n, |b, _| {
            b.iter_batched(
                || clone_nodes(&nodes),
                |clone| {
                    let mut force = ForceManyBody::new().theta(0.9);
                    force.force(black_box(1.0), &clone);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_many_body);
criterion_main!(benches);
