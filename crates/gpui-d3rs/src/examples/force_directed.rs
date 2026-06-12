//! Force-Directed Graph — <https://observablehq.com/@d3/force-directed-graph>
//!
//! Demonstrates: `ForceSimulation` with charge, link, and center forces.
//!
//! Note: The simulation validates convergence rather than exact
//! positions (non-deterministic initial positions).

use crate::force::{ForceCenter, ForceLink, ForceManyBody, Simulation, SimulationNode};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct NodeResult {
    pub id: String,
    pub group: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct LinkData {
    pub source: String,
    pub target: String,
    pub value: usize,
}

/// JSON format matching the miserables.json file.
#[derive(Deserialize)]
struct JsonGraph {
    nodes: Vec<JsonNode>,
    links: Vec<JsonLink>,
}

#[derive(Deserialize)]
struct JsonNode {
    id: String,
    group: usize,
}

#[derive(Deserialize)]
struct JsonLink {
    source: String,
    target: String,
    value: usize,
}

/// Load node and link data from a JSON file (miserables.json format).
pub fn load_json(json_str: &str) -> (Vec<(String, usize)>, Vec<LinkData>) {
    let graph: JsonGraph = serde_json::from_str(json_str).expect("invalid graph JSON");
    let nodes = graph.nodes.into_iter().map(|n| (n.id, n.group)).collect();
    let links = graph
        .links
        .into_iter()
        .map(|l| LinkData {
            source: l.source,
            target: l.target,
            value: l.value,
        })
        .collect();
    (nodes, links)
}

#[derive(Debug)]
pub struct ForceGraphResult {
    pub nodes: Vec<NodeResult>,
    pub links: Vec<LinkData>,
    pub alpha: f64,
    pub iterations: usize,
}

/// Les Miserables-like graph (small subset).
pub fn default_data() -> (Vec<(String, usize)>, Vec<LinkData>) {
    let nodes = vec![
        ("Myriel", 1),
        ("Napoleon", 1),
        ("Labarre", 2),
        ("Valjean", 2),
        ("Marguerite", 3),
        ("Mme.deR", 2),
        ("Isabeau", 2),
        ("Gervais", 2),
        ("Tholomyes", 3),
        ("Listolier", 3),
        ("Fameuil", 3),
        ("Blacheville", 3),
        ("Favourite", 3),
        ("Dahlia", 3),
        ("Zephine", 3),
        ("Fantine", 3),
        ("Cosette", 4),
        ("Javert", 4),
        ("Fauchelevent", 5),
        ("Bamatabois", 5),
    ]
    .into_iter()
    .map(|(n, g)| (n.to_string(), g))
    .collect();

    let links = vec![
        ("Napoleon", "Myriel", 1),
        ("Labarre", "Valjean", 1),
        ("Mme.deR", "Valjean", 1),
        ("Isabeau", "Valjean", 1),
        ("Gervais", "Valjean", 1),
        ("Marguerite", "Valjean", 1),
        ("Tholomyes", "Fantine", 3),
        ("Listolier", "Tholomyes", 4),
        ("Fameuil", "Tholomyes", 4),
        ("Blacheville", "Tholomyes", 4),
        ("Favourite", "Tholomyes", 3),
        ("Dahlia", "Tholomyes", 3),
        ("Zephine", "Tholomyes", 3),
        ("Fantine", "Valjean", 5),
        ("Cosette", "Valjean", 4),
        ("Javert", "Valjean", 6),
        ("Fauchelevent", "Valjean", 2),
        ("Bamatabois", "Valjean", 1),
        ("Cosette", "Javert", 1),
        ("Cosette", "Fantine", 5),
    ]
    .into_iter()
    .map(|(s, t, v)| LinkData {
        source: s.to_string(),
        target: t.to_string(),
        value: v,
    })
    .collect();

    (nodes, links)
}

/// Run force simulation for the given number of iterations.
pub fn compute(
    node_data: &[(String, usize)],
    links: &[LinkData],
    iterations: usize,
) -> ForceGraphResult {
    // Initialize nodes in a phyllotaxis spiral to avoid coincident positions
    // (D3.js does the same — coincident nodes cause force explosions)
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let nodes: Vec<Rc<RefCell<SimulationNode>>> = node_data
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let r = (i as f64).sqrt() * 10.0;
            let theta = i as f64 * golden_angle;
            SimulationNode::new(i, r * theta.cos(), r * theta.sin())
        })
        .collect();

    // Build link index pairs from node names
    let node_names: Vec<&str> = node_data.iter().map(|(n, _)| n.as_str()).collect();
    let link_pairs: Vec<(usize, usize)> = links
        .iter()
        .filter_map(|l| {
            let si = node_names.iter().position(|&n| n == l.source)?;
            let ti = node_names.iter().position(|&n| n == l.target)?;
            Some((si, ti))
        })
        .collect();

    let mut sim = Simulation::new(nodes)
        .force(Box::new(ForceManyBody::new()))
        .force(Box::new(ForceLink::new(link_pairs).distance(30.0)))
        .force(Box::new(ForceCenter::new(0.0, 0.0)));

    for _ in 0..iterations {
        sim.tick();
    }

    let node_results: Vec<NodeResult> = sim
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let n = n.borrow();
            NodeResult {
                id: node_data[i].0.clone(),
                group: node_data[i].1,
                x: n.x,
                y: n.y,
            }
        })
        .collect();

    ForceGraphResult {
        nodes: node_results,
        links: links.to_vec(),
        alpha: sim.alpha,
        iterations,
    }
}
