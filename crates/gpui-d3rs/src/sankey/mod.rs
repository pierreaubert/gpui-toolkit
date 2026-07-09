//! Sankey diagram layout (d3-sankey)
//!
//! Computes node positions and link paths for Sankey flow diagrams.
//! Matches D3's `d3.sankey()` API.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

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

/// Recoverable errors for checked Sankey layout generation.
#[derive(Debug, Clone, PartialEq)]
pub enum SankeyLayoutError {
    /// Numeric layout configuration fields must be finite.
    NonFiniteConfigField { field: &'static str, value: f64 },
    /// Size-like layout configuration fields must be greater than zero.
    NonPositiveConfigField { field: &'static str, value: f64 },
    /// Margin and padding fields cannot be negative.
    NegativeConfigField { field: &'static str, value: f64 },
    /// Margins and node width must leave drawable space.
    InvalidDrawableArea { axis: &'static str, available: f64 },
    /// Node ids must not be empty.
    EmptyNodeName { index: usize },
    /// Checked Sankey node ids must be unique.
    DuplicateNodeName {
        name: String,
        first_index: usize,
        duplicate_index: usize,
    },
    /// Checked Sankey links must reference declared node ids.
    UnknownLinkEndpoint {
        link_index: usize,
        endpoint: &'static str,
        name: String,
    },
    /// Checked Sankey link values must be finite.
    NonFiniteLinkValue { link_index: usize, value: f64 },
    /// Checked Sankey link values cannot be negative.
    NegativeLinkValue { link_index: usize, value: f64 },
}

impl fmt::Display for SankeyLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteConfigField { field, value } => {
                write!(f, "sankey config field {field} is not finite: {value}")
            }
            Self::NonPositiveConfigField { field, value } => {
                write!(
                    f,
                    "sankey config field {field} must be greater than zero: {value}"
                )
            }
            Self::NegativeConfigField { field, value } => {
                write!(f, "sankey config field {field} is negative: {value}")
            }
            Self::InvalidDrawableArea { axis, available } => write!(
                f,
                "sankey {axis} drawable area must be greater than zero: {available}"
            ),
            Self::EmptyNodeName { index } => {
                write!(f, "sankey node name at index {index} is empty")
            }
            Self::DuplicateNodeName {
                name,
                first_index,
                duplicate_index,
            } => write!(
                f,
                "sankey node name {name:?} is duplicated at indexes {first_index} and {duplicate_index}"
            ),
            Self::UnknownLinkEndpoint {
                link_index,
                endpoint,
                name,
            } => write!(
                f,
                "sankey link {link_index} references unknown {endpoint} node {name:?}"
            ),
            Self::NonFiniteLinkValue { link_index, value } => {
                write!(f, "sankey link {link_index} value is not finite: {value}")
            }
            Self::NegativeLinkValue { link_index, value } => {
                write!(f, "sankey link {link_index} value is negative: {value}")
            }
        }
    }
}

impl std::error::Error for SankeyLayoutError {}

/// Node horizontal alignment strategy for Sankey layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SankeyNodeAlign {
    /// Place nodes by their longest distance from any source.
    Left,
    /// Place nodes by their longest distance to any sink.
    Right,
    /// Place sources left, sinks right, and intermediate nodes by depth.
    Center,
    /// Place sinks in the rightmost layer and other nodes by depth.
    Justify,
}

/// Link sorting context passed to Sankey link comparators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SankeyLinkSortContext {
    pub index: usize,
    pub source: usize,
    pub target: usize,
    pub value: f64,
    pub source_layer: usize,
    pub target_layer: usize,
    pub source_y0: f64,
    pub target_y0: f64,
}

pub type SankeyLinkSortFn = fn(&SankeyLinkSortContext, &SankeyLinkSortContext) -> Ordering;

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
    extent: Option<[[f64; 2]; 2]>,
    node_align: SankeyNodeAlign,
    link_sort: Option<SankeyLinkSortFn>,
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
            extent: None,
            node_align: SankeyNodeAlign::Justify,
            link_sort: Some(default_link_sort),
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

    pub fn extent(mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        self.extent = Some([[x0, y0], [x1, y1]]);
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

    pub fn node_align(mut self, align: SankeyNodeAlign) -> Self {
        self.node_align = align;
        self
    }

    pub fn link_sort(mut self, compare: SankeyLinkSortFn) -> Self {
        self.link_sort = Some(compare);
        self
    }

    pub fn link_sort_input_order(mut self) -> Self {
        self.link_sort = None;
        self
    }

    /// Compute the sankey layout from node names and links.
    pub fn compute(&self, node_names: &[String], links: &[SankeyLinkInput]) -> SankeyResult {
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

        self.compute_resolved(node_names, resolved_links)
    }

    /// Compute the Sankey layout after validating configuration and link endpoints.
    ///
    /// Unlike [`Self::compute`], this checked path reports duplicate node ids,
    /// unknown link endpoints, invalid values, and unusable layout geometry
    /// instead of silently dropping links or allowing non-finite layout math.
    pub fn try_compute(
        &self,
        node_names: &[String],
        links: &[SankeyLinkInput],
    ) -> Result<SankeyResult, SankeyLayoutError> {
        self.validate_config()?;
        let resolved_links = self.validate_and_resolve_links(node_names, links)?;

        Ok(self.compute_resolved(node_names, resolved_links))
    }

    fn compute_resolved(
        &self,
        node_names: &[String],
        resolved_links: Vec<(usize, usize, f64)>,
    ) -> SankeyResult {
        let n = node_names.len();

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

        // Assign layers according to the configured alignment.
        let num_layers = max_depth + 1;
        let mut layer = vec![0usize; n];
        for (i, layer_slot) in layer.iter_mut().enumerate() {
            *layer_slot = self
                .aligned_layer(
                    i,
                    max_depth,
                    &depth,
                    &height_val,
                    &source_links,
                    &target_links,
                )
                .min(max_depth);
        }

        // Horizontal positioning
        let (x0, y0, x1, y1) = self.layout_bounds();
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

        // Sort links for visual ordering.
        let mut link_order: Vec<usize> = (0..resolved_links.len()).collect();
        if let Some(compare) = self.link_sort {
            link_order.sort_by(|&a, &b| {
                let a = link_sort_context(a, &resolved_links, &layer, &node_y0);
                let b = link_sort_context(b, &resolved_links, &layer, &node_y0);
                compare(&a, &b)
            });
        }

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

    fn layout_bounds(&self) -> (f64, f64, f64, f64) {
        if let Some([[x0, y0], [x1, y1]]) = self.extent {
            (x0, y0, x1, y1)
        } else {
            (
                self.margin_left,
                self.margin_top,
                self.width - self.margin_right,
                self.height - self.margin_bottom,
            )
        }
    }

    fn aligned_layer(
        &self,
        index: usize,
        max_depth: usize,
        depth: &[usize],
        height: &[usize],
        source_links: &[Vec<usize>],
        target_links: &[Vec<usize>],
    ) -> usize {
        match self.node_align {
            SankeyNodeAlign::Left => depth[index],
            SankeyNodeAlign::Right => max_depth.saturating_sub(height[index]),
            SankeyNodeAlign::Center => {
                if target_links[index].is_empty() {
                    0
                } else if source_links[index].is_empty() {
                    max_depth
                } else {
                    depth[index]
                }
            }
            SankeyNodeAlign::Justify => {
                if source_links[index].is_empty() {
                    max_depth
                } else {
                    depth[index]
                }
            }
        }
    }

    fn validate_config(&self) -> Result<(), SankeyLayoutError> {
        validate_finite_config("width", self.width)?;
        validate_positive_config("width", self.width)?;
        validate_finite_config("height", self.height)?;
        validate_positive_config("height", self.height)?;
        validate_finite_config("node_width", self.node_width)?;
        validate_positive_config("node_width", self.node_width)?;
        validate_finite_config("node_padding", self.node_padding)?;
        validate_non_negative_config("node_padding", self.node_padding)?;

        validate_margin("margin_top", self.margin_top)?;
        validate_margin("margin_right", self.margin_right)?;
        validate_margin("margin_bottom", self.margin_bottom)?;
        validate_margin("margin_left", self.margin_left)?;

        if let Some([[x0, y0], [x1, y1]]) = self.extent {
            validate_finite_config("extent_x0", x0)?;
            validate_finite_config("extent_y0", y0)?;
            validate_finite_config("extent_x1", x1)?;
            validate_finite_config("extent_y1", y1)?;
        }

        let (x0, y0, x1, y1) = self.layout_bounds();
        let drawable_width = x1 - x0 - self.node_width;
        if drawable_width <= 0.0 {
            return Err(SankeyLayoutError::InvalidDrawableArea {
                axis: "x",
                available: drawable_width,
            });
        }

        let drawable_height = y1 - y0;
        if drawable_height <= 0.0 {
            return Err(SankeyLayoutError::InvalidDrawableArea {
                axis: "y",
                available: drawable_height,
            });
        }

        Ok(())
    }

    fn validate_and_resolve_links(
        &self,
        node_names: &[String],
        links: &[SankeyLinkInput],
    ) -> Result<Vec<(usize, usize, f64)>, SankeyLayoutError> {
        let mut name_to_idx = HashMap::with_capacity(node_names.len());
        for (index, name) in node_names.iter().enumerate() {
            if name.is_empty() {
                return Err(SankeyLayoutError::EmptyNodeName { index });
            }
            if let Some(first_index) = name_to_idx.insert(name.as_str(), index) {
                return Err(SankeyLayoutError::DuplicateNodeName {
                    name: name.clone(),
                    first_index,
                    duplicate_index: index,
                });
            }
        }

        let mut resolved_links = Vec::with_capacity(links.len());
        for (link_index, link) in links.iter().enumerate() {
            if !link.value.is_finite() {
                return Err(SankeyLayoutError::NonFiniteLinkValue {
                    link_index,
                    value: link.value,
                });
            }
            if link.value < 0.0 {
                return Err(SankeyLayoutError::NegativeLinkValue {
                    link_index,
                    value: link.value,
                });
            }

            let source = *name_to_idx.get(link.source.as_str()).ok_or_else(|| {
                SankeyLayoutError::UnknownLinkEndpoint {
                    link_index,
                    endpoint: "source",
                    name: link.source.clone(),
                }
            })?;
            let target = *name_to_idx.get(link.target.as_str()).ok_or_else(|| {
                SankeyLayoutError::UnknownLinkEndpoint {
                    link_index,
                    endpoint: "target",
                    name: link.target.clone(),
                }
            })?;

            resolved_links.push((source, target, link.value));
        }

        Ok(resolved_links)
    }
}

fn link_sort_context(
    index: usize,
    resolved_links: &[(usize, usize, f64)],
    layer: &[usize],
    node_y0: &[f64],
) -> SankeyLinkSortContext {
    let (source, target, value) = resolved_links[index];
    SankeyLinkSortContext {
        index,
        source,
        target,
        value,
        source_layer: layer[source],
        target_layer: layer[target],
        source_y0: node_y0[source],
        target_y0: node_y0[target],
    }
}

fn default_link_sort(a: &SankeyLinkSortContext, b: &SankeyLinkSortContext) -> Ordering {
    a.source_layer
        .cmp(&b.source_layer)
        .then(
            a.source_y0
                .partial_cmp(&b.source_y0)
                .unwrap_or(Ordering::Equal),
        )
        .then(
            a.target_y0
                .partial_cmp(&b.target_y0)
                .unwrap_or(Ordering::Equal),
        )
        .then(a.index.cmp(&b.index))
}

fn validate_finite_config(field: &'static str, value: f64) -> Result<(), SankeyLayoutError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SankeyLayoutError::NonFiniteConfigField { field, value })
    }
}

fn validate_positive_config(field: &'static str, value: f64) -> Result<(), SankeyLayoutError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(SankeyLayoutError::NonPositiveConfigField { field, value })
    }
}

fn validate_non_negative_config(field: &'static str, value: f64) -> Result<(), SankeyLayoutError> {
    if value >= 0.0 {
        Ok(())
    } else {
        Err(SankeyLayoutError::NegativeConfigField { field, value })
    }
}

fn validate_margin(field: &'static str, value: f64) -> Result<(), SankeyLayoutError> {
    validate_finite_config(field, value)?;
    validate_non_negative_config(field, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn link(source: &str, target: &str, value: f64) -> SankeyLinkInput {
        SankeyLinkInput {
            source: source.to_string(),
            target: target.to_string(),
            value,
        }
    }

    #[test]
    fn checked_sankey_matches_permissive_for_valid_inputs() {
        let layout = SankeyLayout::new();
        let node_names = names(&["a", "b", "c"]);
        let links = vec![link("a", "b", 2.0), link("b", "c", 1.5)];

        let permissive = layout.compute(&node_names, &links);
        let checked = layout.try_compute(&node_names, &links).unwrap();

        assert_eq!(checked.nodes.len(), permissive.nodes.len());
        assert_eq!(checked.links.len(), permissive.links.len());
        assert_eq!(checked.links[0].source, permissive.links[0].source);
        assert_eq!(checked.links[1].target, permissive.links[1].target);
        assert!(checked.nodes.iter().all(|node| {
            node.x0.is_finite()
                && node.x1.is_finite()
                && node.y0.is_finite()
                && node.y1.is_finite()
                && node.value.is_finite()
        }));
    }

    #[test]
    fn checked_sankey_rejects_invalid_layout_config() {
        let node_names = names(&["a", "b"]);
        let links = vec![link("a", "b", 1.0)];

        assert_eq!(
            SankeyLayout::new()
                .node_width(0.0)
                .try_compute(&node_names, &links)
                .unwrap_err(),
            SankeyLayoutError::NonPositiveConfigField {
                field: "node_width",
                value: 0.0
            }
        );

        assert_eq!(
            SankeyLayout::new()
                .margins(0.0, 60.0, 0.0, 60.0)
                .width(100.0)
                .node_width(10.0)
                .try_compute(&node_names, &links)
                .unwrap_err(),
            SankeyLayoutError::InvalidDrawableArea {
                axis: "x",
                available: -30.0
            }
        );
    }

    #[test]
    fn extent_overrides_width_height_margins_for_node_positions() {
        let node_names = names(&["a", "b"]);
        let links = vec![link("a", "b", 1.0)];

        let result = SankeyLayout::new()
            .extent(10.0, 20.0, 210.0, 120.0)
            .node_width(20.0)
            .try_compute(&node_names, &links)
            .unwrap();

        let source = &result.nodes[0];
        let sink = &result.nodes[1];
        assert_eq!(source.x0, 10.0);
        assert_eq!(source.x1, 30.0);
        assert_eq!(source.y0, 20.0);
        assert_eq!(sink.x0, 190.0);
        assert_eq!(sink.x1, 210.0);
    }

    #[test]
    fn checked_sankey_rejects_invalid_extent() {
        let node_names = names(&["a", "b"]);
        let links = vec![link("a", "b", 1.0)];

        assert_eq!(
            SankeyLayout::new()
                .extent(0.0, f64::INFINITY, 10.0, 10.0)
                .try_compute(&node_names, &links)
                .unwrap_err(),
            SankeyLayoutError::NonFiniteConfigField {
                field: "extent_y0",
                value: f64::INFINITY
            }
        );

        assert_eq!(
            SankeyLayout::new()
                .extent(10.0, 0.0, 0.0, 10.0)
                .node_width(10.0)
                .try_compute(&node_names, &links)
                .unwrap_err(),
            SankeyLayoutError::InvalidDrawableArea {
                axis: "x",
                available: -20.0
            }
        );
    }

    #[test]
    fn node_align_left_keeps_short_sinks_near_their_depth() {
        let node_names = names(&["a", "b", "c", "d"]);
        let links = vec![
            link("a", "b", 1.0),
            link("a", "c", 1.0),
            link("c", "d", 1.0),
        ];

        let justified = SankeyLayout::new()
            .node_align(SankeyNodeAlign::Justify)
            .try_compute(&node_names, &links)
            .unwrap();
        let left = SankeyLayout::new()
            .node_align(SankeyNodeAlign::Left)
            .try_compute(&node_names, &links)
            .unwrap();

        assert_eq!(justified.nodes[1].layer, 2);
        assert_eq!(left.nodes[1].layer, 1);
        assert!(left.nodes[1].x0 < justified.nodes[1].x0);
    }

    #[test]
    fn link_sort_customizes_link_vertical_order() {
        fn descending_value(a: &SankeyLinkSortContext, b: &SankeyLinkSortContext) -> Ordering {
            b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal)
        }

        let node_names = names(&["a", "b", "c"]);
        let links = vec![link("a", "b", 1.0), link("a", "c", 3.0)];

        let result = SankeyLayout::new()
            .link_sort(descending_value)
            .try_compute(&node_names, &links)
            .unwrap();

        let low_value = &result.links[0];
        let high_value = &result.links[1];
        assert_eq!(low_value.target, 1);
        assert_eq!(high_value.target, 2);
        assert!(high_value.y0 < low_value.y0);
    }

    #[test]
    fn checked_sankey_rejects_duplicate_or_empty_node_names() {
        let links = vec![link("a", "b", 1.0)];

        assert_eq!(
            SankeyLayout::new()
                .try_compute(&names(&["a", "", "b"]), &links)
                .unwrap_err(),
            SankeyLayoutError::EmptyNodeName { index: 1 }
        );

        assert_eq!(
            SankeyLayout::new()
                .try_compute(&names(&["a", "b", "a"]), &links)
                .unwrap_err(),
            SankeyLayoutError::DuplicateNodeName {
                name: "a".to_string(),
                first_index: 0,
                duplicate_index: 2
            }
        );
    }

    #[test]
    fn checked_sankey_rejects_unknown_link_endpoints() {
        let node_names = names(&["a", "b"]);
        let links = vec![link("a", "missing", 1.0)];

        assert_eq!(
            SankeyLayout::new()
                .try_compute(&node_names, &links)
                .unwrap_err(),
            SankeyLayoutError::UnknownLinkEndpoint {
                link_index: 0,
                endpoint: "target",
                name: "missing".to_string()
            }
        );

        assert_eq!(
            SankeyLayout::new().compute(&node_names, &links).links.len(),
            0
        );
    }

    #[test]
    fn checked_sankey_rejects_invalid_link_values() {
        let node_names = names(&["a", "b"]);

        assert!(matches!(
            SankeyLayout::new().try_compute(&node_names, &[link("a", "b", f64::NAN)]),
            Err(SankeyLayoutError::NonFiniteLinkValue {
                link_index: 0,
                value,
            }) if value.is_nan()
        ));

        assert_eq!(
            SankeyLayout::new()
                .try_compute(&node_names, &[link("a", "b", -1.0)])
                .unwrap_err(),
            SankeyLayoutError::NegativeLinkValue {
                link_index: 0,
                value: -1.0
            }
        );
    }
}
