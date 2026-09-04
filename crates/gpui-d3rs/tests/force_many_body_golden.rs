//! Golden parity: d3rs many-body force vs d3-force reference values.
//!
//! Fixture `golden/force/many_body.json` is generated from the original
//! D3.js library — regenerate with `cd golden && node force_manybody.mjs`
//! (d3 7.9.0: `d3.forceSimulation` + `d3.forceManyBody`, strength -30,
//! theta 0.9, distanceMin 0, distanceMax Infinity, alpha 1 decaying per
//! tick, velocity multiplier 0.6). This test replays the same node lattice,
//! configuration, and tick counts through `d3rs::force::Simulation` and
//! asserts numerical parity for both the Barnes-Hut path (`theta(0.9)`,
//! mirroring d3's own approximation) and the exact brute-force fallback
//! (`theta = Infinity`, the d3rs default).

use d3rs::force::{ForceManyBody, Simulation, SimulationNode};
use serde::Deserialize;
use std::fs;

const TOL_BH: f64 = 1e-9;
const TOL_BRUTE: f64 = 5.0;

#[derive(Debug, Deserialize)]
struct InputNode {
    index: usize,
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct OutputNode {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn run_simulation(inputs: &[InputNode], ticks: usize, theta: f64) -> Vec<(f64, f64, f64, f64)> {
    let nodes: Vec<_> = inputs
        .iter()
        .map(|n| SimulationNode::new(n.index, n.x, n.y))
        .collect();
    let force = ForceManyBody::new().theta(theta).distance_min(0.0);
    let mut sim = Simulation::new(nodes.clone()).force(Box::new(force));
    for _ in 0..ticks {
        sim.tick();
    }
    nodes
        .iter()
        .map(|n| {
            let n = n.borrow();
            (n.x, n.y, n.vx, n.vy)
        })
        .collect()
}

fn max_diff(actual: &[(f64, f64, f64, f64)], expected: &[OutputNode]) -> f64 {
    actual
        .iter()
        .zip(expected.iter())
        .flat_map(|((x, y, vx, vy), e)| {
            [
                (x - e.x).abs(),
                (y - e.y).abs(),
                (vx - e.vx).abs(),
                (vy - e.vy).abs(),
            ]
        })
        .fold(0.0f64, f64::max)
}

#[test]
fn many_body_matches_d3_force_reference() {
    let content = fs::read_to_string("golden/force/many_body.json").expect("golden file not found");
    let golden: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(golden["module"], "d3-force");
    assert_eq!(golden["function"], "forceManyBody");
    assert_eq!(golden["d3_version"], "7.9.0");

    for case in golden["test_cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let ticks = case["ticks"].as_u64().unwrap() as usize;
        let inputs: Vec<InputNode> = serde_json::from_value(case["inputs"].clone()).unwrap();
        let expected: Vec<OutputNode> = serde_json::from_value(case["outputs"].clone()).unwrap();
        assert_eq!(inputs.len(), expected.len());

        let bh = run_simulation(&inputs, ticks, 0.9);
        let brute = run_simulation(&inputs, ticks, f64::INFINITY);
        println!("case {name}: max|bh - d3| = {:e}", max_diff(&bh, &expected));
        println!(
            "case {name}: max|brute - d3| = {:e}",
            max_diff(&brute, &expected)
        );
        assert!(
            max_diff(&bh, &expected) < TOL_BH,
            "case '{name}': Barnes-Hut diverged from d3-force reference"
        );
        assert!(
            max_diff(&brute, &expected) < TOL_BRUTE,
            "case '{name}': brute force diverged from d3-force reference"
        );
    }
}
