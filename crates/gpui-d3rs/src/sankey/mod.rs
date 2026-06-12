//! Sankey diagram layout (d3-sankey)
//!
//! Computes node positions and link paths for Sankey flow diagrams.
//! Matches D3's `d3.sankey()` API.

use std::collections::HashMap;

/// A node in the sankey diagram after layout.
#[derive(Debug, Clone)]
pub struct SankeyNode {
    pub id: String,
    pub index: usize,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
    pub value: f64,
    pub depth: usize,  // distance from source
    pub height: usize, // distance from sink
    pub layer: usize,  // assigned column
}

/// A link in the sankey diagram after layout.
#[derive(Debug, Clone)]
pub struct SankeyLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
    pub y0: f64, // y position at source node
    pub y1: f64, // y position at target node
    pub width: f64,
    pub path: String, // SVG path string (cubic Bézier)
}

/// Input link (string-based source/target).
#[derive(Debug, Clone)]
pub struct SankeyLinkInput {
    pub source: String,
    pub target: String,
    pub value: f64,
}

/// Sankey layout result.
#[derive(Debug)]
pub struct SankeyResult {
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
}

/// Sankey layout configuration.
pub struct SankeyLayout {
    width: f64,
    height: f64,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
    node_width: f64,
    node_padding: f64,
    iterations: usize,
}

impl Default for SankeyLayout {
    fn default() -> Self {
        Self {
            width: 928.0,
            height: 600.0,
            margin_top: 5.0,
            margin_right: 1.0,
            margin_bottom: 5.0,
            margin_left: 1.0,
            node_width: 15.0,
            node_padding: 10.0,
            iterations: 6,
        }
    }
}

impl SankeyLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }

    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }

    pub fn margins(mut self, top: f64, right: f64, bottom: f64, left: f64) -> Self {
        self.margin_top = top;
        self.margin_right = right;
        self.margin_bottom = bottom;
        self.margin_left = left;
        self
    }

    pub fn node_width(mut self, w: f64) -> Self {
        self.node_width = w;
        self
    }

    pub fn node_padding(mut self, p: f64) -> Self {
        self.node_padding = p;
        self
    }

    pub fn iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    /// Compute the sankey layout from node names and links.
    pub fn compute(&self, node_names: &[String], links: &[SankeyLinkInput]) -> SankeyResult {
        let n = node_names.len();
        let name_to_idx: HashMap<&str, usize> = node_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.as_str(), i))
            .collect();

        // Resolve links to indices
        let resolved_links: Vec<(usize, usize, f64)> = links
            .iter()
            .filter_map(|l| {
                let si = name_to_idx.get(l.source.as_str())?;
                let ti = name_to_idx.get(l.target.as_str())?;
                Some((*si, *ti, l.value))
            })
            .collect();

        // Compute node values (sum of connected link values)
        let mut node_values = vec![0.0f64; n];
        let mut source_links: Vec<Vec<usize>> = vec![Vec::new(); n]; // link indices by source
        let mut target_links: Vec<Vec<usize>> = vec![Vec::new(); n]; // link indices by target
        for (li, &(si, ti, _)) in resolved_links.iter().enumerate() {
            source_links[si].push(li);
            target_links[ti].push(li);
        }
        // Value = max(sum of outgoing, sum of incoming)
        for i in 0..n {
            let out_sum: f64 = source_links[i].iter().map(|&li| resolved_links[li].2).sum();
            let in_sum: f64 = target_links[i].iter().map(|&li| resolved_links[li].2).sum();
            node_values[i] = out_sum.max(in_sum);
        }

        // Compute depth (longest path from any source)
        // Bellman-Ford with iteration cap to avoid infinite loops on cyclic input
        let mut depth = vec![0usize; n];
        for _ in 0..n {
            let mut changed = false;
            for &(si, ti, _) in &resolved_links {
                if depth[ti] <= depth[si] {
                    depth[ti] = depth[si] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Compute height (longest path to any sink)
        let max_depth = depth.iter().copied().max().unwrap_or(0);
        let mut height_val = vec![0usize; n];
        for _ in 0..n {
            let mut changed = false;
            for &(si, ti, _) in &resolved_links {
                if height_val[si] <= height_val[ti] {
                    height_val[si] = height_val[ti] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Assign layers — "justify" alignment: spread across max_depth layers
        let num_layers = max_depth + 1;
        let mut layer = vec![0usize; n];
        for i in 0..n {
            if source_links[i].is_empty() {
                // Sink nodes: place at rightmost layer
                layer[i] = max_depth;
            } else if target_links[i].is_empty() {
                // Source nodes: place at layer 0
                layer[i] = 0;
            } else {
                // Justify: use depth
                layer[i] = depth[i];
            }
        }
        // Sinks without outgoing links go to the right
        for i in 0..n {
            if source_links[i].is_empty() {
                layer[i] = max_depth;
            }
        }

        // Horizontal positioning
        let x0 = self.margin_left;
        let x1 = self.width - self.margin_right;
        let dx = if num_layers > 1 {
            (x1 - x0 - self.node_width) / (num_layers - 1) as f64
        } else {
            0.0
        };

        // Collect nodes per layer
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); num_layers];
        for i in 0..n {
            layers[layer[i]].push(i);
        }

        // Sort nodes within each layer by their incoming link position for less crossing
        for layer_nodes in &mut layers {
            layer_nodes.sort_by(|&a, &b| {
                let a_target_avg = if target_links[a].is_empty() {
                    0.0
                } else {
                    let sum: f64 = target_links[a]
                        .iter()
                        .map(|&li| resolved_links[li].0 as f64)
                        .sum();
                    sum / target_links[a].len() as f64
                };
                let b_target_avg = if target_links[b].is_empty() {
                    0.0
                } else {
                    let sum: f64 = target_links[b]
                        .iter()
                        .map(|&li| resolved_links[li].0 as f64)
                        .sum();
                    sum / target_links[b].len() as f64
                };
                a_target_avg
                    .partial_cmp(&b_target_avg)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Vertical positioning: distribute nodes within each layer
        let y0 = self.margin_top;
        let y1 = self.height - self.margin_bottom;
        let available_height = y1 - y0;

        let mut node_y0 = vec![0.0f64; n];
        let mut node_y1 = vec![0.0f64; n];

        for layer_nodes in &layers {
            if layer_nodes.is_empty() {
                continue;
            }
            let total_value: f64 = layer_nodes.iter().map(|&i| node_values[i]).sum();
            let total_padding = self.node_padding * (layer_nodes.len() as f64 - 1.0).max(0.0);
            let k = if total_value > 0.0 {
                (available_height - total_padding) / total_value
            } else {
                1.0
            };

            let mut y = y0;
            for &ni in layer_nodes {
                node_y0[ni] = y;
                let h = node_values[ni] * k;
                node_y1[ni] = y + h;
                y += h + self.node_padding;
            }
        }

        // Iterative relaxation to reduce link crossings
        for _ in 0..self.iterations {
            // Relax nodes based on linked node positions
            for layer_nodes in &layers {
                for &ni in layer_nodes {
                    let mut weighted_y = 0.0;
                    let mut total_weight = 0.0;

                    // Pull toward source positions
                    for &li in &target_links[ni] {
                        let si = resolved_links[li].0;
                        let center = (node_y0[si] + node_y1[si]) / 2.0;
                        let w = resolved_links[li].2;
                        weighted_y += center * w;
                        total_weight += w;
                    }
                    // Pull toward target positions
                    for &li in &source_links[ni] {
                        let ti = resolved_links[li].1;
                        let center = (node_y0[ti] + node_y1[ti]) / 2.0;
                        let w = resolved_links[li].2;
                        weighted_y += center * w;
                        total_weight += w;
                    }

                    if total_weight > 0.0 {
                        let target_center = weighted_y / total_weight;
                        let current_center = (node_y0[ni] + node_y1[ni]) / 2.0;
                        let h = node_y1[ni] - node_y0[ni];
                        let dy = (target_center - current_center) * 0.5; // damped
                        node_y0[ni] = (node_y0[ni] + dy).max(y0).min(y1 - h);
                        node_y1[ni] = node_y0[ni] + h;
                    }
                }

                // Resolve overlaps within layer
                let mut sorted_nodes: Vec<usize> = layer_nodes.clone();
                sorted_nodes.sort_by(|&a, &b| {
                    node_y0[a]
                        .partial_cmp(&node_y0[b])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let mut prev_bottom = y0;
                for &ni in &sorted_nodes {
                    let overlap = prev_bottom - node_y0[ni];
                    if overlap > 0.0 {
                        let h = node_y1[ni] - node_y0[ni];
                        node_y0[ni] += overlap;
                        node_y1[ni] = node_y0[ni] + h;
                    }
                    prev_bottom = node_y1[ni] + self.node_padding;
                }
            }
        }

        // Build SankeyNode results
        let nodes: Vec<SankeyNode> = (0..n)
            .map(|i| SankeyNode {
                id: node_names[i].clone(),
                index: i,
                x0: x0 + layer[i] as f64 * dx,
                x1: x0 + layer[i] as f64 * dx + self.node_width,
                y0: node_y0[i],
                y1: node_y1[i],
                value: node_values[i],
                depth: depth[i],
                height: height_val[i],
                layer: layer[i],
            })
            .collect();

        // Compute link positions
        // For each node, track how much vertical space has been used for links
        let mut source_y_used = vec![0.0f64; n]; // cumulative y offset at source
        let mut target_y_used = vec![0.0f64; n]; // cumulative y offset at target

        // Sort links by target node position for better visual ordering
        let mut link_order: Vec<usize> = (0..resolved_links.len()).collect();
        link_order.sort_by(|&a, &b| {
            let (sa, ta, _) = resolved_links[a];
            let (sb, tb, _) = resolved_links[b];
            let ya = node_y0[ta];
            let yb = node_y0[tb];
            layer[sa]
                .cmp(&layer[sb])
                .then(
                    node_y0[sa]
                        .partial_cmp(&node_y0[sb])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(ya.partial_cmp(&yb).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut sankey_links: Vec<SankeyLink> = vec![
            SankeyLink {
                source: 0,
                target: 0,
                value: 0.0,
                y0: 0.0,
                y1: 0.0,
                width: 0.0,
                path: String::new(),
            };
            resolved_links.len()
        ];

        for &li in &link_order {
            let (si, ti, val) = resolved_links[li];
            let source_node = &nodes[si];
            let target_node = &nodes[ti];

            // Link width proportional to value, scaled to node height
            let source_height = source_node.y1 - source_node.y0;
            let source_k = if source_node.value > 0.0 {
                source_height / source_node.value
            } else {
                0.0
            };
            let width = val * source_k;

            let link_y0 = source_node.y0 + source_y_used[si] + width / 2.0;
            source_y_used[si] += width;

            let target_height = target_node.y1 - target_node.y0;
            let target_k = if target_node.value > 0.0 {
                target_height / target_node.value
            } else {
                0.0
            };
            let target_width = val * target_k;
            let link_y1 = target_node.y0 + target_y_used[ti] + target_width / 2.0;
            target_y_used[ti] += target_width;

            // D3 sankey link path: horizontal cubic Bézier
            let sx = source_node.x1;
            let tx = target_node.x0;
            let cx = (sx + tx) / 2.0;
            let path = format!(
                "M{sx},{y0}C{cx},{y0},{cx},{y1},{tx},{y1}",
                sx = sx,
                y0 = link_y0,
                cx = cx,
                y1 = link_y1,
                tx = tx
            );

            sankey_links[li] = SankeyLink {
                source: si,
                target: ti,
                value: val,
                y0: link_y0,
                y1: link_y1,
                width,
                path,
            };
        }

        SankeyResult {
            nodes,
            links: sankey_links,
        }
    }
}
