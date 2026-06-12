//! Sankey Diagram — <https://observablehq.com/@d3/sankey>
//!
//! Demonstrates: `SankeyLayout` with energy flow data.

use crate::sankey::{SankeyLayout, SankeyLinkInput, SankeyResult};
use serde::Deserialize;

/// JSON format matching energy.json.
#[derive(Deserialize)]
struct JsonGraph {
    nodes: Vec<JsonNode>,
    links: Vec<JsonLink>,
}

#[derive(Deserialize)]
struct JsonNode {
    name: String,
    #[allow(dead_code)]
    category: String,
}

#[derive(Deserialize)]
struct JsonLink {
    source: String,
    target: String,
    value: f64,
}

/// Load energy flow data from JSON (energy.json format).
pub fn load_json(json_str: &str) -> (Vec<String>, Vec<SankeyLinkInput>) {
    let graph: JsonGraph = serde_json::from_str(json_str).expect("invalid energy JSON");
    let names: Vec<String> = graph.nodes.into_iter().map(|n| n.name).collect();
    let links: Vec<SankeyLinkInput> = graph
        .links
        .into_iter()
        .map(|l| SankeyLinkInput {
            source: l.source,
            target: l.target,
            value: l.value,
        })
        .collect();
    (names, links)
}

/// Compute sankey layout from node names and links.
pub fn compute(names: &[String], links: &[SankeyLinkInput]) -> SankeyResult {
    SankeyLayout::new()
        .width(928.0)
        .height(600.0)
        .margins(5.0, 1.0, 5.0, 1.0)
        .node_width(15.0)
        .node_padding(10.0)
        .compute(names, links)
}
